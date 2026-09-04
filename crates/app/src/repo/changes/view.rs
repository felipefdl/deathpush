use std::collections::HashSet;

use deathpush_core::config::layout::MainView;
use deathpush_core::session::types::{Intent, SessionActions};
use deathpush_core::types::{RepoOperationState, ResourceGroupKind};
use gpui_kit::base::ResizableState;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{Input, InputEvent, InputState, TextareaState};
use gpui_kit::*;

use super::banner::render_banner;
use super::commit_box::{self, render_commit_box};
use super::filter::{self, FILTER_DEBOUNCE_MS};
use super::groups::{FileRow, GroupBody, GroupId, assemble_groups, render_groups};
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
}

pub struct ChangesView {
  pub(crate) model: Entity<RepoModel>,
  pub(crate) layout: Entity<LayoutModel>,
  pub(crate) commit: Entity<TextareaState>,
  filter: Entity<InputState>,
  filter_text: String,
  filter_generation: u64,
  commit_generation: u64,
  pub(crate) committing: bool,
  pub(crate) selected: HashSet<(ResourceGroupKind, String)>,
  pub(crate) anchor: Option<(GroupId, usize)>,
  pub(crate) groups_state: Entity<ResizableState>,
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
        .default_value(commit_message)
    });
    let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter files..."));
    if !file_filter.is_empty() {
      filter.update(cx, |state, cx| state.set_value(file_filter.clone(), window, cx));
    }

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
    cx.subscribe(&model, |this, model, event: &RepoEvent, cx| {
      this.committing = false;
      if matches!(event, RepoEvent::Changed) {
        let message = model.read(cx).state().commit_message.clone();
        let current = this.commit.read(cx).value().to_string();
        let pending = this.commit_generation != 0;
        let handle = this.window_handle;
        let commit = this.commit.clone();
        let _ = handle.update(cx, |_, window, cx| {
          commit.update(cx, |state, cx| {
            let focused = state.focus_handle(cx).is_focused(window);
            if commit_box::should_sync_commit_message(&current, &message, focused, pending) {
              state.set_value(message, window, cx);
            }
          });
        });
      }
      cx.notify();
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
      committing: false,
      selected: HashSet::new(),
      anchor: None,
      groups_state: cx.new(|_| ResizableState::default()),
      window_handle: window.window_handle(),
      focus_handle: cx.focus_handle(),
    }
  }

  pub fn focus_commit(&self, window: &mut Window, cx: &mut App) {
    self.commit.update(cx, |state, cx| state.focus(window, cx));
  }

  pub fn commit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.committing = true;
    cx.notify();
    self.send(Intent::Commit { confirmed: false }, window, cx);
  }

  pub fn filter_text(&self) -> &str {
    &self.filter_text
  }

  pub(crate) fn send(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
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
    if rows.is_empty() {
      return;
    }
    let last = rows.len() - 1;
    let start = anchor_index.min(index).min(last);
    let end = anchor_index.max(index).min(last);
    self.selected.clear();
    for row in &rows[start..=end] {
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
    for path in self.target_paths(row) {
      self.send(Intent::OpenFileHistory { path }, window, cx);
    }
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
        },
        groups,
      )
    };
    let mut root = div()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus_handle)
      .key_context("Changes")
      .on_action(cx.listener(|this, _: &CommitFromBox, window, cx| this.commit(window, cx)))
      .on_action(cx.listener(|this, _: &CommitAmendMode, window, cx| {
        this.send(Intent::SetAmend { enabled: true }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndPush, window, cx| {
        this.committing = true;
        cx.notify();
        this.send(Intent::CommitAndPush { confirmed: false }, window, cx);
      }))
      .on_action(cx.listener(|this, _: &CommitAndSync, window, cx| {
        this.committing = true;
        cx.notify();
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
    root
  }
}
