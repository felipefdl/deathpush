use std::collections::HashSet;
use std::rc::Rc;

use deathpush_core::config::layout::MainView;
use gpui_kit::base::{Tree, TreeEvent, TreeItem, TreeState};
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::*;

use super::icons::IconKind;
use super::model::{EditState, ExplorerEvent, ExplorerModel, Node};
use super::rows::{RowPaint, render_row};
use crate::actions::OpenRepository;
use crate::config::AppConfig;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::{ActivePalette, hsla};

pub struct ExplorerView {
  model: Entity<ExplorerModel>,
  repo: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  tree: Entity<TreeState>,
  filter: Entity<InputState>,
  tree_focus: FocusHandle,
}

impl ExplorerView {
  pub fn new(
    model: Entity<ExplorerModel>,
    repo: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let filter = cx.new(|cx| InputState::new(window, cx).placeholder("Filter files..."));
    let tree = cx.new(|cx| TreeState::new(cx));
    let tree_focus = cx.focus_handle();

    cx.subscribe(&filter, |this, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let value = input.read(cx).value().to_string();
        this.model.update(cx, |model, cx| model.set_filter(value, cx));
      }
    })
    .detach();
    cx.subscribe(&tree, |this, _, event: &TreeEvent, cx| match event {
      TreeEvent::Expanded(id) => this.model.update(cx, |model, cx| model.expand(id.as_str(), cx)),
      TreeEvent::Collapsed(id) => this.model.update(cx, |model, cx| model.collapse(id.as_str(), cx)),
    })
    .detach();
    cx.subscribe_in(
      &model,
      window,
      |this, _, event: &ExplorerEvent, window, cx| match event {
        ExplorerEvent::Changed => this.rebuild_tree(cx),
        ExplorerEvent::OpenFile { path, line } => this.open_file(path, *line, window, cx),
        ExplorerEvent::Error(_) | ExplorerEvent::Toast(_) | ExplorerEvent::Renamed { .. } => {}
      },
    )
    .detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    cx.observe(&repo, |_, _, cx| cx.notify()).detach();
    cx.observe_global::<AppConfig>(|_, cx| cx.notify()).detach();

    Self {
      model,
      repo,
      layout,
      tree,
      filter,
      tree_focus,
    }
  }

  pub fn owns_focus(&self, window: &Window, cx: &App) -> bool {
    self.filter.read(cx).focus_handle(cx).is_focused(window) || self.tree_focus.is_focused(window)
  }

  pub fn open_file(&mut self, path: &str, line: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
    let _ = window;
    self.model.update(cx, |model, cx| model.select(path, false, false, cx));
    let id = SharedString::from(path.to_string());
    self.tree.update(cx, |tree, cx| {
      tree.reveal_item(&id, ScrollStrategy::Center, cx);
    });
    self.repo.update(cx, |model, cx| model.record_recent_file(path, cx));
    self.layout.update(cx, |layout, cx| {
      layout.dock_terminal(cx);
      layout.select_main_view(MainView::File, cx);
    });
    self.repo.update(cx, |model, cx| model.open_file(path, line, cx));
  }

  pub(crate) fn on_row_mouse_down(
    &mut self,
    path: &str,
    is_directory: bool,
    event: &MouseDownEvent,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let extend = event.modifiers.secondary();
    let range = event.modifiers.shift;
    self.model.update(cx, |model, cx| model.select(path, extend, range, cx));
    if !is_directory && !extend && !range {
      let already_open = self
        .repo
        .read(cx)
        .state()
        .open_file
        .as_ref()
        .is_some_and(|open| open.path == path);
      if already_open {
        self.layout.update(cx, |layout, cx| layout.dock_terminal(cx));
      } else {
        self.open_file(path, None, window, cx);
      }
    }
    if let Some(focused) = window.focused(cx) {
      self.tree_focus = focused;
    }
  }

  fn create_entry(&mut self, is_directory: bool, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| {
      model.begin_create("", is_directory, cx);
      let Some(EditState::Creating { name, .. }) = model.edit.as_ref() else {
        return;
      };
      let name = name.clone();
      model.commit_edit(name, window, cx);
    });
  }

  fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
    let (items, selected) = {
      let model = self.model.read(cx);
      (
        items_from_nodes(&model.roots, &model.expanded, &model.filter),
        model.selected.first().cloned(),
      )
    };
    self.tree.update(cx, |tree, cx| {
      tree.set_items(items, cx);
      if let Some(path) = selected {
        let id = SharedString::from(path);
        if let Some(index) = tree.index_of(&id) {
          tree.set_selected_index(Some(index), cx);
        }
      }
    });
  }
}

fn stub_id(path: &str) -> String {
  format!("{path}\u{2060}")
}

fn is_stub_id(id: &str) -> bool {
  id.ends_with('\u{2060}')
}

fn items_from_nodes(nodes: &[Node], expanded: &HashSet<String>, filter: &str) -> Vec<TreeItem> {
  nodes
    .iter()
    .filter_map(|node| node_item(node, expanded, filter))
    .collect()
}

fn node_item(node: &Node, expanded: &HashSet<String>, filter: &str) -> Option<TreeItem> {
  let filtering = !filter.is_empty();
  if filtering && !node_visible(node, filter) {
    return None;
  }
  let mut item = TreeItem::new(node.path.clone(), node.name.clone());
  if node.is_directory {
    let mut children: Vec<TreeItem> = match &node.children {
      Some(children) => children
        .iter()
        .filter_map(|child| node_item(child, expanded, filter))
        .collect(),
      None => Vec::new(),
    };
    if children.is_empty() {
      children.push(TreeItem::new(stub_id(&node.path), ""));
    }
    item = item
      .children(children)
      .expanded(filtering || expanded.contains(&node.path));
  }
  Some(item)
}

fn node_visible(node: &Node, filter: &str) -> bool {
  let needle = filter.to_ascii_lowercase();
  if node.name.to_ascii_lowercase().contains(&needle) || node.path.to_ascii_lowercase().contains(&needle) {
    return true;
  }
  node
    .children
    .as_ref()
    .is_some_and(|children| children.iter().any(|child| node_visible(child, filter)))
}

fn tool(id: &'static str, path: &'static str, tooltip: &'static str) -> Button {
  Button::new(id)
    .ghost()
    .xsmall()
    .w(px(22.0))
    .h(px(22.0))
    .icon(Icon::empty().path(path))
    .tooltip(tooltip)
}

impl Render for ExplorerView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let repo_open = self.repo.read(cx).state().root().is_some();
    let palette = cx.global::<ActivePalette>().0;
    let density = AppConfig::get(cx).settings.ui.tree_density;
    let kind = IconKind::from(AppConfig::get(cx).settings.ui.tree_icons);
    let mut root = div().size_full().flex().flex_col();

    if !repo_open {
      return root
        .child(render_empty_header(&palette))
        .child(render_empty_body(&palette));
    }

    root = root.child(self.render_header(&palette, cx));
    let status = self.repo.read(cx).state().status.clone();
    let rows = Rc::new(self.model.read(cx).visible_rows(status.as_ref()));
    let view = cx.weak_entity();
    root.child(
      div().flex_1().min_h_0().w_full().track_focus(&self.tree_focus).child(
        Tree::new(&self.tree)
          .item(move |_, entry, entry_state, _, _| {
            let path = entry.item().id.as_str();
            if is_stub_id(path) {
              return div().h(px(0.0)).into_any_element();
            }
            let Some(row) = rows.iter().find(|row| row.path == path) else {
              return div().into_any_element();
            };
            render_row(
              row,
              &RowPaint {
                kind,
                density,
                palette,
                selected: row.selected || entry_state.is_selected(),
              },
              view.clone(),
            )
          })
          .flex_1()
          .min_h_0()
          .w_full(),
      ),
    )
  }
}

impl ExplorerView {
  fn render_header(&self, palette: &deathpush_core::theme::UiPalette, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .h(px(35.0))
      .flex_shrink_0()
      .flex()
      .items_center()
      .gap_1()
      .px_2()
      .border_b_1()
      .border_color(hsla(palette.border))
      .child(
        div().flex_1().min_w_0().child(
          Input::new(&self.filter)
            .small()
            .h(px(22.0))
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
      )
      .child(
        tool("explorer-new-file", "icons/new-file.svg", "New File")
          .on_click(cx.listener(|this, _, window, cx| this.create_entry(false, window, cx))),
      )
      .child(
        tool("explorer-new-folder", "icons/new-folder.svg", "New Folder")
          .on_click(cx.listener(|this, _, window, cx| this.create_entry(true, window, cx))),
      )
      .child(
        tool("explorer-refresh", "icons/refresh.svg", "Refresh Explorer").on_click(cx.listener(|this, _, _, cx| {
          this.model.update(cx, |model, cx| model.load(cx));
        })),
      )
  }
}

fn render_empty_header(palette: &deathpush_core::theme::UiPalette) -> impl IntoElement {
  div()
    .h(px(35.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .px_2()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      div()
        .text_size(px(11.0))
        .font_weight(FontWeight::BOLD)
        .text_color(hsla(palette.muted_foreground))
        .child("EXPLORER"),
    )
}

fn render_empty_body(palette: &deathpush_core::theme::UiPalette) -> impl IntoElement {
  div()
    .flex_1()
    .min_h_0()
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
      Button::new("explorer-open-repo")
        .outline()
        .icon(Icon::empty().path("icons/folder.svg"))
        .label("Open Repository")
        .on_click(|_, window, cx| window.dispatch_action(Box::new(OpenRepository), cx)),
    )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  use deathpush_core::Core;
  use deathpush_core::session::types::{
    OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{RepoOperationState, StatusPhase};
  use gpui_kit::TestAppContext;

  use crate::config::AppConfig;
  use crate::repo::layout_model::LayoutModel;
  use crate::repo::model::RepoModel;

  fn snapshot(root: &str) -> SessionSnapshot {
    SessionSnapshot {
      session_generation: 1,
      session_revision: 1,
      status_generation: 1,
      status_revision: 1,
      repo: SessionRepo {
        root: root.into(),
        head_branch: Some("main".into()),
        head_commit: Some("abc".into()),
        ahead: 0,
        behind: 0,
        operation_state: RepoOperationState::None,
        phase: StatusPhase::Settled,
      },
      groups: vec![],
      selection: SessionSelection::default(),
      scm: SessionScm::default(),
      actions: SessionActions {
        can_commit: false,
        commit_label: "Commit".into(),
        commit_destructive: false,
        can_stage_all: false,
        can_unstage_all: false,
        can_discard_all: false,
        discard_all_destructive: false,
        sync: SyncAction {
          enabled: false,
          kind: SyncKind::Fetch,
          destructive: false,
        },
        operation: OperationActions {
          continue_op: false,
          abort: false,
          skip: false,
          abort_destructive: false,
        },
      },
      last_commit: None,
      branches: vec![],
      stashes: vec![],
      tags: vec![],
      commit_log: vec![],
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  #[gpui_kit::test]
  fn explorer_view_renders(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let layout_dir = config_dir.path().to_path_buf();
    let root = layout_dir.to_string_lossy().into_owned();
    let window = cx.add_window({
      let core = core.clone();
      let snapshot = snapshot(&root);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        let explorer_model = cx.new(|cx| {
          let mut explorer = ExplorerModel::new(model.read(cx).core(), session, root.clone());
          explorer.load(cx);
          explorer
        });
        ExplorerView::new(explorer_model, model, layout, window, cx)
      }
    });
    window
      .update(cx, |view, window, cx| {
        window.refresh();
        assert!(view.filter.read(cx).value().is_empty());
        assert!(
          !view.owns_focus(window, cx),
          "new must not steal tree focus; owns_focus is tree, filter, or edit field"
        );
      })
      .unwrap();
  }
}
