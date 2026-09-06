use std::collections::HashSet;

use deathpush_core::config::layout::MainView;
use deathpush_core::session::types::{Intent, SessionActions};
use deathpush_core::types::{RepoOperationState, ResourceGroupKind};
use gpui_kit::base::ResizableState;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::banner::render_banner;
use super::commit_box::{self, render_commit_box};
use super::filter::{self, FILTER_DEBOUNCE_MS};
use super::groups::{FileRow, GroupBody, GroupId, assemble_groups, render_groups, tree_range, visible_tree};
use super::overflow::{BranchListMode, OverflowItem, dispatch_item, filter_branches};
use super::toolbar::render_toolbar;
use crate::actions::*;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::{RepoEvent, RepoModel};
use crate::theme::{ActivePalette, hsla};

pub(crate) struct ChangesChrome {
  pub actions: Option<SessionActions>,
  pub network_busy: bool,
  pub ahead: usize,
  pub behind: usize,
  pub operation_state: RepoOperationState,
  pub amend_mode: bool,
  pub head_branch: Option<String>,
  pub committing: bool,
}

pub struct ChangesView {
  pub(crate) model: Entity<RepoModel>,
  pub(crate) layout: Entity<LayoutModel>,
  pub(crate) commit: Entity<TextareaState>,
  filter: Entity<InputState>,
  filter_text: String,
  filter_generation: u64,
  commit_generation: u64,
  core_commit_message: String,
  pub(crate) selected: HashSet<(ResourceGroupKind, String)>,
  pub(crate) anchor: Option<(GroupId, usize)>,
  pub(crate) collapsed_folders: HashSet<(GroupId, String)>,
  pub(crate) groups_state: Entity<ResizableState>,
  branch_list: Option<BranchListMode>,
  branch_query: Entity<InputState>,
  window_handle: AnyWindowHandle,
  focus_handle: FocusHandle,
}

impl ChangesView {
  pub fn new(
    model: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let state = model.read(cx).state();
    let commit_message = state.commit_message.clone();
    let file_filter = state.file_filter.clone();
    let commit = cx.new(|cx| {
      TextareaState::new(window, cx)
        .placeholder("commit message")
        .auto_grow(2, 9)
        .default_value(commit_message.clone())
    });
    let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter files..."));
    if !file_filter.is_empty() {
      filter.update(cx, |state, cx| state.set_value(file_filter.clone(), window, cx));
    }
    let branch_query = cx.new(|cx| InputState::new(window, cx).placeholder("Select a branch..."));

    cx.subscribe(&commit, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.commit_generation + 1;
        filter::debounce(cx, &mut this.commit_generation, FILTER_DEBOUNCE_MS, move |this, cx| {
          if this.commit_generation != token {
            return;
          }
          this.commit_generation = 0;
          let message = this.commit.read(cx).value().to_string();
          this.dispatch_intent(Intent::SetCommitMessage { message }, cx);
        });
      }
    })
    .detach();
    cx.subscribe(&filter, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.filter_generation + 1;
        filter::debounce(cx, &mut this.filter_generation, FILTER_DEBOUNCE_MS, move |this, cx| {
          if this.filter_generation != token {
            return;
          }
          let filter = this.filter.read(cx).value().to_string();
          this.filter_text = filter.clone();
          this.dispatch_intent(Intent::SetFileFilter { filter }, cx);
        });
      }
    })
    .detach();
    cx.subscribe_in(&model, window, |this, model, event: &RepoEvent, window, cx| {
      if matches!(event, RepoEvent::Changed) {
        let message = model.read(cx).state().commit_message.clone();
        let previous = std::mem::replace(&mut this.core_commit_message, message.clone());
        let current = this.commit.read(cx).value().to_string();
        let pending = this.commit_generation != 0;
        this.commit.update(cx, |state, cx| {
          let focused = state.focus_handle(cx).is_focused(window);
          if commit_box::should_sync_commit_message(&current, &message, &previous, focused, pending) {
            state.set_value(message, window, cx);
          }
        });
      }
      cx.notify();
    })
    .detach();
    cx.subscribe(&branch_query, |_, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        cx.notify();
      }
    })
    .detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();

    Self {
      model,
      layout,
      commit,
      filter,
      filter_text: file_filter,
      filter_generation: 0,
      commit_generation: 0,
      core_commit_message: commit_message,
      selected: HashSet::new(),
      anchor: None,
      collapsed_folders: HashSet::new(),
      groups_state: cx.new(|_| ResizableState::default()),
      branch_list: None,
      branch_query,
      window_handle: window.window_handle(),
      focus_handle: cx.focus_handle(),
    }
  }

  pub fn focus_commit(&self, window: &mut Window, cx: &mut App) {
    self.commit.update(cx, |state, cx| state.focus(window, cx));
  }

  pub(crate) fn owns_focus(&self, window: &Window, cx: &App) -> bool {
    self.focus_handle.is_focused(window)
      || self.commit.read(cx).focus_handle(cx).is_focused(window)
      || self.filter.read(cx).focus_handle(cx).is_focused(window)
  }

  pub fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.send(Intent::Commit { confirmed: false }, window, cx);
  }

  pub fn filter_text(&self) -> &str {
    &self.filter_text
  }

  pub(crate) fn send(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }

  pub(crate) fn activate_overflow(&mut self, item: OverflowItem, window: &mut Window, cx: &mut Context<Self>) {
    if dispatch_item(&self.model, item, window, cx) {
      return;
    }
    match item {
      OverflowItem::MergeBranch => self.open_branch_list(BranchListMode::Merge, window, cx),
      OverflowItem::RebaseBranch => self.open_branch_list(BranchListMode::Rebase, window, cx),
      OverflowItem::StageAll => self.send(Intent::StageAll, window, cx),
      OverflowItem::UnstageAll => self.send(Intent::UnstageAll, window, cx),
      OverflowItem::Stash => self.send(
        Intent::StashSave {
          include_untracked: false,
          staged_only: false,
          message: None,
        },
        window,
        cx,
      ),
      OverflowItem::StashPop => self.send(Intent::StashPop { index: 0 }, window, cx),
      OverflowItem::UndoCommit => self.send(Intent::UndoCommit { confirmed: false }, window, cx),
      OverflowItem::OpenRepository => window.dispatch_action(Box::new(OpenRepository), cx),
      OverflowItem::CloneRepository => window.dispatch_action(Box::new(CloneRepository), cx),
      _ => {}
    }
  }

  pub(crate) fn open_branch_list(&mut self, mode: BranchListMode, window: &mut Window, cx: &mut Context<Self>) {
    self.branch_list = Some(mode);
    self.branch_query.update(cx, |state, cx| {
      state.set_value("", window, cx);
      state.focus(window, cx);
    });
    cx.notify();
  }

  fn close_branch_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.branch_list.is_none() {
      return;
    }
    self.branch_list = None;
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  fn confirm_branch_list(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(mode) = self.branch_list else {
      return;
    };
    let query = self.branch_query.read(cx).value().to_string();
    let name = {
      let state = self.model.read(cx).state();
      let current = state.head_branch();
      filter_branches(&state.branches, current, &query)
        .first()
        .map(|branch| branch.name.clone())
    };
    let Some(name) = name else {
      return;
    };
    self.pick_branch(mode, name, window, cx);
  }

  fn pick_branch(&mut self, mode: BranchListMode, name: String, window: &mut Window, cx: &mut Context<Self>) {
    let intent = mode.intent(name);
    self.close_branch_list(window, cx);
    self.send(intent, window, cx);
  }

  fn dispatch_intent(&self, intent: Intent, cx: &mut Context<Self>) {
    let model = self.model.clone();
    let _ = self.window_handle.update(cx, |_, window, cx| {
      model.update(cx, |model, cx| model.dispatch(intent, window, cx));
    });
  }

  pub(crate) fn on_file_click(
    &mut self,
    row: FileRow,
    group_id: GroupId,
    index: usize,
    event: &ClickEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let modifiers = event.modifiers();
    if modifiers.shift && self.anchor.is_some_and(|(group, _)| group == group_id) {
      self.select_range(group_id, index, cx);
      return;
    }
    if modifiers.secondary() {
      let key = (row.group_kind, row.path.clone());
      if !self.selected.remove(&key) {
        self.selected.insert(key);
      }
      if self.anchor.is_none() {
        self.anchor = Some((group_id, index));
      }
      cx.notify();
      return;
    }
    self.select_file(&row, group_id, index, window, cx);
  }

  pub(crate) fn select_file(
    &mut self,
    row: &FileRow,
    group_id: GroupId,
    index: usize,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.selected.clear();
    self.selected.insert((row.group_kind, row.path.clone()));
    self.anchor = Some((group_id, index));
    self.layout.update(cx, |layout, cx| {
      layout.select_main_view(MainView::Changes, cx);
      layout.dock_terminal(cx);
    });
    self.send(
      Intent::OpenScmDiff {
        path: row.path.clone(),
        staged: row.staged,
        group_kind: Some(row.group_kind),
      },
      window,
      cx,
    );
    cx.notify();
  }

  fn select_range(&mut self, group_id: GroupId, index: usize, cx: &mut Context<Self>) {
    let Some((anchor_group, anchor_index)) = self.anchor else {
      return;
    };
    if anchor_group != group_id {
      return;
    }
    let groups = assemble_groups(self.model.read(cx).state(), self.filter_text());
    let Some(group) = groups.iter().find(|group| group.id == group_id) else {
      return;
    };
    let GroupBody::Files(rows) = &group.body else {
      return;
    };
    let tree = visible_tree(rows, group_id, &self.collapsed_folders);
    let indices = tree_range(&tree, anchor_index, index);
    self.selected.clear();
    for index in indices {
      let row = &rows[index];
      self.selected.insert((row.group_kind, row.path.clone()));
    }
    cx.notify();
  }

  fn target_keys(&self, row: &FileRow) -> Vec<(ResourceGroupKind, String, bool)> {
    let key = (row.group_kind, row.path.clone());
    if self.selected.contains(&key) {
      self
        .selected
        .iter()
        .map(|(kind, path)| (*kind, path.clone(), *kind == ResourceGroupKind::Index))
        .collect()
    } else {
      vec![(row.group_kind, row.path.clone(), row.staged)]
    }
  }

  fn target_paths(&self, row: &FileRow) -> Vec<String> {
    self.target_keys(row).into_iter().map(|(_, path, _)| path).collect()
  }

  pub(crate) fn menu_open_changes(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    let group_id = match row.group_kind {
      ResourceGroupKind::Merge => GroupId::Merge,
      ResourceGroupKind::Index => GroupId::Staged,
      ResourceGroupKind::WorkingTree | ResourceGroupKind::Untracked => GroupId::Changes,
    };
    let index = assemble_groups(self.model.read(cx).state(), self.filter_text())
      .iter()
      .find(|group| group.id == group_id)
      .and_then(|group| match &group.body {
        GroupBody::Files(rows) => rows
          .iter()
          .position(|item| item.path == row.path && item.group_kind == row.group_kind),
        _ => None,
      })
      .unwrap_or(0);
    self.select_file(row, group_id, index, window, cx);
  }

  pub(crate) fn menu_open_file(&mut self, row: &FileRow, _: &mut Window, cx: &mut Context<Self>) {
    for path in self.target_paths(row) {
      self.model.update(cx, |model, cx| model.open_in_editor(path, cx));
    }
  }

  pub(crate) fn menu_show_history(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    self
      .layout
      .update(cx, |layout, cx| layout.select_main_view(MainView::History, cx));
    self.send(Intent::OpenFileHistory { path: row.path.clone() }, window, cx);
  }

  pub(crate) fn menu_stage(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    self.send(
      Intent::Stage {
        paths: self.target_paths(row),
      },
      window,
      cx,
    );
  }

  pub(crate) fn menu_unstage(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    self.send(
      Intent::Unstage {
        paths: self.target_paths(row),
      },
      window,
      cx,
    );
  }

  pub(crate) fn menu_discard(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    self.send(
      Intent::Discard {
        paths: self.target_paths(row),
        confirmed: false,
      },
      window,
      cx,
    );
  }

  pub(crate) fn menu_copy_path(&mut self, row: &FileRow, _: &mut Window, cx: &mut Context<Self>) {
    let root = self.model.read(cx).root_path();
    let text = self
      .target_paths(row)
      .into_iter()
      .map(|path| match &root {
        Some(root) => root.join(&path).to_string_lossy().into_owned(),
        None => path,
      })
      .collect::<Vec<_>>()
      .join("\n");
    cx.write_to_clipboard(ClipboardItem::new_string(text));
  }

  pub(crate) fn menu_copy_relative(&mut self, row: &FileRow, _: &mut Window, cx: &mut Context<Self>) {
    cx.write_to_clipboard(ClipboardItem::new_string(self.target_paths(row).join("\n")));
  }

  pub(crate) fn menu_reveal(&mut self, row: &FileRow, _: &mut Window, cx: &mut Context<Self>) {
    for path in self.target_paths(row) {
      self
        .model
        .update(cx, |model, cx| model.reveal_in_file_manager(path, cx));
    }
  }

  pub(crate) fn menu_trash(&mut self, row: &FileRow, window: &mut Window, cx: &mut Context<Self>) {
    for path in self.target_paths(row) {
      self.send(Intent::DeleteFile { path, confirmed: false }, window, cx);
    }
  }

  fn render_branch_list(&self, mode: BranchListMode, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let query = self.branch_query.read(cx).value().to_string();
    let (branches, current) = {
      let state = self.model.read(cx).state();
      (state.branches.clone(), state.head_branch().map(str::to_string))
    };
    let matches = filter_branches(&branches, current.as_deref(), &query);
    let rows: Vec<AnyElement> = matches
      .iter()
      .map(|branch| {
        let name = branch.name.clone();
        let icon = if branch.is_remote {
          "icons/cloud.svg"
        } else {
          "icons/git-branch.svg"
        };
        let view = cx.weak_entity();
        let pick_mode = mode;
        div()
          .id(SharedString::from(format!("scm-branch-{name}")))
          .h(px(26.0))
          .flex_shrink_0()
          .flex()
          .items_center()
          .gap_1()
          .px_2()
          .cursor_pointer()
          .hover(|el| el.bg(hsla(palette.list_hover)))
          .on_click(move |_, window, cx| {
            let _ = view.update(cx, |this, cx| this.pick_branch(pick_mode, name.clone(), window, cx));
          })
          .child(
            svg()
              .path(icon)
              .size(px(14.0))
              .text_color(hsla(palette.muted_foreground)),
          )
          .child(
            div()
              .min_w_0()
              .flex_1()
              .overflow_hidden()
              .text_ellipsis()
              .text_size(px(13.0))
              .text_color(hsla(palette.foreground))
              .child(branch.name.clone()),
          )
          .into_any_element()
      })
      .collect();
    let empty = matches.is_empty();
    div()
      .id("scm-branch-list")
      .key_context("BranchList")
      .occlude()
      .absolute()
      .top(px(35.0))
      .right_2()
      .w(px(260.0))
      .flex()
      .flex_col()
      .bg(hsla(palette.sidebar))
      .border_1()
      .border_color(hsla(palette.border))
      .rounded_md()
      .shadow_lg()
      .on_action(cx.listener(|this, _: &Confirm, window, cx| this.confirm_branch_list(window, cx)))
      .on_action(cx.listener(|this, _: &Cancel, window, cx| this.close_branch_list(window, cx)))
      .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close_branch_list(window, cx)))
      .child(
        div()
          .h(px(22.0))
          .flex_shrink_0()
          .flex()
          .items_center()
          .px_2()
          .text_size(px(11.0))
          .font_weight(FontWeight::BOLD)
          .text_color(hsla(palette.muted_foreground))
          .child(mode.header().to_uppercase()),
      )
      .child(
        div().px_2().pb_2().child(
          Input::new(&self.branch_query)
            .small()
            .h(px(26.0))
            .w_full()
            .rounded_md()
            .bg(hsla(palette.input))
            .cleanable(true),
        ),
      )
      .child(
        div()
          .id("scm-branch-list-rows")
          .max_h(px(260.0))
          .overflow_y_scroll()
          .flex()
          .flex_col()
          .when(empty, |el| {
            el.child(
              div()
                .px_2()
                .py_1()
                .text_size(px(12.0))
                .text_color(hsla(palette.muted_foreground))
                .child("No matching branches"),
            )
          })
          .when(!empty, |el| el.children(rows)),
      )
  }

  fn render_empty_repo(cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    div()
      .size_full()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_3()
      .child(
        div()
          .text_size(px(13.0))
          .text_color(hsla(palette.muted_foreground))
          .child("No repository open"),
      )
      .child(
        Button::new("open-repo")
          .outline()
          .label("Open Repository")
          .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenRepository), cx)),
      )
  }

  fn render_watermark(cx: &App) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    div()
      .flex_1()
      .min_h_0()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_2()
      .child(
        svg()
          .path("brand/deathpush.svg")
          .size(px(48.0))
          .text_color(hsla(palette.mark))
          .opacity(0.12),
      )
      .child(
        div()
          .text_size(px(13.0))
          .text_color(hsla(palette.foreground))
          .opacity(0.18)
          .child("No changes"),
      )
  }
}

impl Render for ChangesView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let (repo_open, has_changes, chrome, groups) = {
      let state = self.model.read(cx).state();
      let status = state.status.as_ref();
      let groups = assemble_groups(state, &self.filter_text);
      (
        status.is_some(),
        state.has_changes(),
        ChangesChrome {
          actions: state.actions.clone(),
          network_busy: state.network_busy(),
          ahead: status.map(|status| status.ahead).unwrap_or(0),
          behind: status.map(|status| status.behind).unwrap_or(0),
          operation_state: status
            .map(|status| status.operation_state)
            .unwrap_or(RepoOperationState::None),
          amend_mode: state.amend_mode,
          head_branch: state.head_branch().map(str::to_string),
          committing: state.committing,
        },
        groups,
      )
    };
    let branch_list = self.branch_list;
    let mut root = div()
      .relative()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus_handle)
      .key_context("Changes")
      .on_action(cx.listener(|this, _: &CommitFromBox, window, cx| {
        if !this.commit.read(cx).focus_handle(cx).is_focused(window) {
          return;
        }
        this.commit(window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAmendMode, window, cx| {
        this.send(Intent::SetAmend { enabled: true }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndPush, window, cx| {
        this.send(Intent::CommitAndPush { confirmed: false }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndSync, window, cx| {
        this.send(Intent::CommitAndSync { confirmed: false }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &RefreshStatus, window, cx| {
        this.send(Intent::RefreshStatus, window, cx);
        this.model.update(cx, |model, cx| model.refresh_nested_repositories(cx));
      }))
      .on_action(cx.listener(|this, _: &OperationContinue, window, cx| {
        this.send(Intent::OperationContinue, window, cx);
      }))
      .on_action(cx.listener(|this, _: &OperationSkip, window, cx| {
        this.send(Intent::OperationSkip, window, cx);
      }))
      .on_action(cx.listener(|this, _: &OperationAbort, window, cx| {
        this.send(Intent::OperationAbort, window, cx);
      }))
      .on_action(cx.listener(|this, _: &FocusCommitMessage, window, cx| {
        this.focus_commit(window, cx);
      }))
      .on_action(cx.listener(|this, _: &MergeBranchPicker, window, cx| {
        this.open_branch_list(BranchListMode::Merge, window, cx);
      }))
      .on_action(cx.listener(|this, _: &RebaseBranchPicker, window, cx| {
        this.open_branch_list(BranchListMode::Rebase, window, cx);
      }));

    if !repo_open {
      return root.child(Self::render_empty_repo(cx));
    }

    root = root.child(render_toolbar(&chrome, cx));
    if let Some(banner) = render_banner(&chrome, cx) {
      root = root.child(banner);
    }
    root = root.child(render_commit_box(self, &chrome, window, cx));

    if has_changes {
      let palette = cx.global::<ActivePalette>().0;
      root = root.child(
        div().px_2().pb_2().child(
          Input::new(&self.filter)
            .small()
            .h(px(26.0))
            .w_full()
            .rounded_md()
            .bg(hsla(palette.input))
            .cleanable(true)
            .prefix(
              svg()
                .path("icons/search.svg")
                .size(px(14.0))
                .text_color(hsla(palette.muted_foreground)),
            ),
        ),
      );
    }
    if groups.is_empty() {
      if self.filter_text.is_empty() {
        root = root.child(Self::render_watermark(cx));
      }
    } else {
      root = root.child(render_groups(self, &groups, window, cx));
    }
    if let Some(mode) = branch_list {
      root = root.child(self.render_branch_list(mode, cx));
    }
    root
  }
}
