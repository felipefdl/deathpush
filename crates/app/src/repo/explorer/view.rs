use std::collections::HashSet;
use std::rc::Rc;

use deathpush_core::config::layout::MainView;
use gpui_kit::base::{Tree, TreeEvent, TreeItem, TreeState};
use gpui_kit::component::Icon;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::ContextMenuExt;
use gpui_kit::*;

use super::conflicts::{CONFLICT_KEEP_BOTH, CONFLICT_REPLACE, CONFLICT_TITLE, ConflictChoice, is_conflict_error};
use super::edit::{stem_range, valid_entry_name};
use super::icons::IconKind;
use super::menus::{ItemMenu, blank_menu_items};
use super::model::{ClipboardOp, EditState, ExplorerEvent, ExplorerModel, Node, parent_path};
use super::rows::{DragEntry, RowPaint, drop_ignored, fill_menu, render_row};
use crate::actions::*;
use crate::config::AppConfig;
use crate::keymap::CONTEXT_EXPLORER;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::{ActivePalette, hsla};

#[derive(Clone)]
enum PendingTransfer {
  Paste { into: String },
  Move { source: String, into: String },
  Import { sources: Vec<String> },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ConflictUi {
  Toast,
  Dialog,
}

pub struct ExplorerView {
  model: Entity<ExplorerModel>,
  repo: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  tree: Entity<TreeState>,
  filter: Entity<InputState>,
  tree_focus: FocusHandle,
  edit_field: Option<Entity<InputState>>,
  edit_for: Option<EditState>,
  edit_sub: Option<Subscription>,
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
        ExplorerEvent::Changed => {
          this.sync_edit_field(window, cx);
          this.rebuild_tree(cx);
        }
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
      edit_field: None,
      edit_for: None,
      edit_sub: None,
    }
  }

  pub fn owns_focus(&self, window: &Window, cx: &App) -> bool {
    self.filter.read(cx).focus_handle(cx).is_focused(window)
      || self.tree_focus.is_focused(window)
      || self
        .edit_field
        .as_ref()
        .is_some_and(|field| field.read(cx).focus_handle(cx).is_focused(window))
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
    let creating = self
      .model
      .read(cx)
      .edit
      .as_ref()
      .is_some_and(|edit| matches!(edit, EditState::Creating { .. }) && edit.path() == path);
    if !is_directory && !extend && !range && !creating {
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
  }

  fn capture_tree_focus(&mut self, window: &Window, cx: &App) {
    if let Some(focused) = window.focused(cx) {
      self.tree_focus = focused;
    }
  }

  fn create_entry(&mut self, is_directory: bool, _: &mut Window, cx: &mut Context<Self>) {
    self
      .model
      .update(cx, |model, cx| model.begin_create("", is_directory, cx));
  }

  fn rebuild_tree(&mut self, cx: &mut Context<Self>) {
    let (items, selected) = {
      let model = self.model.read(cx);
      let roots = model.display_roots();
      (
        items_from_nodes(&roots, &model.expanded, &model.filter),
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

  fn sync_edit_field(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let edit = self.model.read(cx).edit.clone();
    if self.edit_for == edit && self.edit_field.is_some() == edit.is_some() {
      return;
    }
    let had_field = self.edit_field.is_some();
    self.edit_for = edit.clone();
    self.edit_field = None;
    self.edit_sub = None;
    let Some(edit) = edit else {
      if had_field {
        self.focus_tree(window, cx);
      }
      return;
    };
    let name = match &edit {
      EditState::Creating { name, .. } | EditState::Renaming { name, .. } => name.clone(),
    };
    let renaming = matches!(edit, EditState::Renaming { .. });
    let field = cx.new(|cx| {
      let mut state = InputState::new(window, cx);
      state.set_value(name.clone(), window, cx);
      if renaming {
        state.set_selected_range(stem_range(&name), cx);
      } else {
        state.select_all(window, cx);
      }
      state
    });
    self.edit_sub = Some(
      cx.subscribe_in(&field, window, |this, _, event, window, cx| match event {
        InputEvent::PressEnter { .. } => this.finish_edit(true, window, cx),
        InputEvent::Blur => this.finish_edit(false, window, cx),
        _ => {}
      }),
    );
    self.edit_field = Some(field);
    cx.defer_in(window, |this, window, cx| {
      if let Some(field) = this.edit_field.clone() {
        field.update(cx, |state, cx| state.focus(window, cx));
      }
    });
  }

  fn finish_edit(&mut self, from_enter: bool, window: &mut Window, cx: &mut Context<Self>) {
    let Some(field) = self.edit_field.clone() else {
      return;
    };
    let name = field.read(cx).value().to_string();
    if !valid_entry_name(&name) {
      if from_enter {
        self.model.update(cx, |_, cx| {
          cx.emit(ExplorerEvent::Toast("Invalid file name".into()));
        });
      } else {
        self.stop_edit(window, cx);
      }
      return;
    }
    self.model.update(cx, |model, cx| model.commit_edit(name, window, cx));
    if self.model.read(cx).edit.is_none() {
      self.edit_sub = None;
    }
  }

  fn stop_edit(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.edit_field = None;
    self.edit_sub = None;
    self.edit_for = None;
    self.model.update(cx, |model, cx| model.cancel_edit(cx));
    self.focus_tree(window, cx);
  }

  fn focus_tree(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.tree.update(cx, |tree, cx| tree.focus(window, cx));
    cx.defer_in(window, |this, window, cx| this.capture_tree_focus(window, cx));
  }

  pub(crate) fn on_item_menu(
    &mut self,
    item: ItemMenu,
    path: &str,
    is_directory: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let parent = if is_directory {
      path.to_string()
    } else {
      parent_path(path)
    };
    match item {
      ItemMenu::NewFile => self
        .model
        .update(cx, |model, cx| model.begin_create(&parent, false, cx)),
      ItemMenu::NewFolder => self.model.update(cx, |model, cx| model.begin_create(&parent, true, cx)),
      ItemMenu::OpenInEditor => self.model.update(cx, |model, cx| model.open_in_editor(path, cx)),
      ItemMenu::Rename => self.model.update(cx, |model, cx| model.begin_rename(path, cx)),
      ItemMenu::Duplicate => self.model.update(cx, |model, cx| model.duplicate(path, cx)),
      ItemMenu::Cut => self.model.update(cx, |model, cx| {
        model.select(path, false, false, cx);
        model.mark(ClipboardOp::Cut, cx);
      }),
      ItemMenu::Copy => self.model.update(cx, |model, cx| {
        model.select(path, false, false, cx);
        model.mark(ClipboardOp::Copy, cx);
      }),
      ItemMenu::Paste => self.run_transfer(
        PendingTransfer::Paste { into: parent },
        None,
        ConflictUi::Dialog,
        window,
        cx,
      ),
      ItemMenu::RevealInFinder => self.model.update(cx, |model, cx| model.reveal(path, cx)),
      ItemMenu::CopyPath => {
        let text = match self.repo.read(cx).root_path() {
          Some(root) => root.join(path).to_string_lossy().into_owned(),
          None => path.to_string(),
        };
        cx.write_to_clipboard(ClipboardItem::new_string(text));
      }
      ItemMenu::CopyRelativePath => cx.write_to_clipboard(ClipboardItem::new_string(path.to_string())),
      ItemMenu::MoveToTrash => {
        let repo = self.repo.clone();
        self.model.update(cx, |model, cx| model.delete(path, &repo, window, cx));
      }
      ItemMenu::AddToGitignore => {
        let repo = self.repo.clone();
        self
          .model
          .update(cx, |model, cx| model.add_to_gitignore(path, &repo, window, cx));
      }
    }
  }

  pub(crate) fn drop_entry(&mut self, entry: &DragEntry, into: &str, window: &mut Window, cx: &mut Context<Self>) {
    if drop_ignored(&entry.path, entry.is_directory, into) {
      return;
    }
    self.run_transfer(
      PendingTransfer::Move {
        source: entry.path.clone(),
        into: into.to_string(),
      },
      None,
      ConflictUi::Dialog,
      window,
      cx,
    );
  }

  pub fn import_external(&mut self, sources: Vec<String>, window: &mut Window, cx: &mut Context<Self>) {
    if sources.is_empty() {
      return;
    }
    self.run_transfer(
      PendingTransfer::Import { sources },
      None,
      ConflictUi::Dialog,
      window,
      cx,
    );
  }

  fn sync_keyboard_selection(&mut self, cx: &mut Context<Self>) {
    let tree_id = self
      .tree
      .read(cx)
      .selected_entry()
      .map(|entry| entry.item().id.to_string());
    let selected = self.model.read(cx).selected.clone();
    let Some(path) = keyboard_path_to_select(tree_id.as_deref(), &selected) else {
      return;
    };
    self.model.update(cx, |model, cx| model.select(&path, false, false, cx));
  }

  fn rename_selected(&mut self, cx: &mut Context<Self>) {
    self.sync_keyboard_selection(cx);
    let Some(path) = self.model.read(cx).anchor.clone() else {
      return;
    };
    self.model.update(cx, |model, cx| model.begin_rename(&path, cx));
  }

  fn delete_selected(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.sync_keyboard_selection(cx);
    let paths = {
      let model = self.model.read(cx);
      if model.selected.is_empty() {
        model.anchor.clone().into_iter().collect::<Vec<_>>()
      } else {
        model.selected.clone()
      }
    };
    let repo = self.repo.clone();
    for path in paths {
      self
        .model
        .update(cx, |model, cx| model.delete(&path, &repo, window, cx));
    }
  }

  fn paste_keyboard(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.sync_keyboard_selection(cx);
    let Some(into) = self.model.read(cx).paste_target() else {
      return;
    };
    self.run_transfer(PendingTransfer::Paste { into }, None, ConflictUi::Toast, window, cx);
  }

  fn run_transfer(
    &mut self,
    pending: PendingTransfer,
    on_conflict: Option<&'static str>,
    ui: ConflictUi,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let done = move |result: Result<(), String>| {
      let _ = tx.send(result);
    };
    self.model.update(cx, |model, cx| match &pending {
      PendingTransfer::Paste { into } => model.paste(into, on_conflict, window, cx, done),
      PendingTransfer::Move { source, into } => model.move_into(source, into, on_conflict, window, cx, done),
      PendingTransfer::Import { sources } => model.import(sources.clone(), on_conflict, window, cx, done),
    });
    cx.spawn_in(window, async move |this, cx| {
      let Ok(result) = rx.await else {
        return;
      };
      let _ = this.update_in(cx, |this, window, cx| {
        this.finish_transfer(pending, result, ui, window, cx);
      });
    })
    .detach();
  }

  fn finish_transfer(
    &mut self,
    pending: PendingTransfer,
    result: Result<(), String>,
    ui: ConflictUi,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    match result {
      Ok(()) => {}
      Err(message) if ui == ConflictUi::Dialog && is_conflict_error(&message) => {
        self.prompt_replace(pending, window, cx);
      }
      Err(message) => {
        let conflict = is_conflict_error(&message);
        self.model.update(cx, |_, cx| {
          if conflict {
            cx.emit(ExplorerEvent::Toast(message));
          } else {
            cx.emit(ExplorerEvent::Error(message));
          }
        });
      }
    }
  }

  fn prompt_replace(&mut self, pending: PendingTransfer, window: &mut Window, cx: &mut Context<Self>) {
    let answer = window.prompt(
      PromptLevel::Warning,
      CONFLICT_TITLE,
      Some(CONFLICT_REPLACE),
      &["Replace", "Cancel"],
      cx,
    );
    cx.spawn_in(window, async move |this, cx| {
      let choice = match answer.await {
        Ok(0) => ConflictChoice::Replace,
        Ok(1) => ConflictChoice::KeepBoth,
        _ => ConflictChoice::Cancel,
      };
      match choice {
        ConflictChoice::Replace => {
          let _ = this.update_in(cx, |this, window, cx| {
            this.run_transfer(pending, Some("replace"), ConflictUi::Toast, window, cx);
          });
        }
        ConflictChoice::KeepBoth => {
          let _ = this.update_in(cx, |this, window, cx| {
            this.prompt_keep_both(pending, window, cx);
          });
        }
        ConflictChoice::Cancel => {}
      }
    })
    .detach();
  }

  fn prompt_keep_both(&mut self, pending: PendingTransfer, window: &mut Window, cx: &mut Context<Self>) {
    let answer = window.prompt(
      PromptLevel::Info,
      CONFLICT_TITLE,
      Some(CONFLICT_KEEP_BOTH),
      &["Keep Both", "Cancel"],
      cx,
    );
    cx.spawn_in(window, async move |this, cx| {
      let choice = match answer.await {
        Ok(0) => ConflictChoice::KeepBoth,
        _ => ConflictChoice::Cancel,
      };
      if choice == ConflictChoice::KeepBoth {
        let _ = this.update_in(cx, |this, window, cx| {
          this.run_transfer(pending, Some("keep-both"), ConflictUi::Toast, window, cx);
        });
      }
    })
    .detach();
  }
}

fn stub_id(path: &str) -> String {
  format!("{path}\u{2060}")
}

fn is_stub_id(id: &str) -> bool {
  id.ends_with('\u{2060}')
}

fn keyboard_path_to_select(tree_id: Option<&str>, selected: &[String]) -> Option<String> {
  let id = tree_id.filter(|id| !is_stub_id(id))?;
  if selected.iter().any(|path| path == id) {
    None
  } else {
    Some(id.to_string())
  }
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
    let mut root = div()
      .size_full()
      .flex()
      .flex_col()
      .key_context(CONTEXT_EXPLORER)
      .on_action(cx.listener(|this, _: &ExplorerRename, _, cx| this.rename_selected(cx)))
      .on_action(cx.listener(|this, _: &ExplorerDelete, window, cx| this.delete_selected(window, cx)))
      .on_action(cx.listener(|this, _: &ExplorerCut, _, cx| {
        this.sync_keyboard_selection(cx);
        this.model.update(cx, |model, cx| model.mark(ClipboardOp::Cut, cx));
      }))
      .on_action(cx.listener(|this, _: &ExplorerCopy, _, cx| {
        this.sync_keyboard_selection(cx);
        this.model.update(cx, |model, cx| model.mark(ClipboardOp::Copy, cx));
      }))
      .on_action(cx.listener(|this, _: &ExplorerPaste, window, cx| this.paste_keyboard(window, cx)))
      .on_action(cx.listener(|this, _: &Cancel, window, cx| {
        if this.edit_field.is_some() {
          this.stop_edit(window, cx);
        }
      }));

    if !repo_open {
      return root
        .child(render_empty_header(&palette))
        .child(render_empty_body(&palette));
    }

    root = root.child(self.render_header(&palette, cx));
    let status = self.repo.read(cx).state().status.clone();
    let rows = Rc::new(self.model.read(cx).visible_rows(status.as_ref()));
    let view = cx.weak_entity();
    let has_mark = self.model.read(cx).clipboard.is_some();
    let edit = self.model.read(cx).edit.clone();
    let edit_field = self.edit_field.clone();
    let hover = hsla(palette.list_hover);
    let blank_view = view.clone();
    let drop_view = view.clone();
    root.child(
      div()
        .id("explorer-tree")
        .flex_1()
        .min_h_0()
        .w_full()
        .track_focus(&self.tree_focus)
        .on_mouse_down(
          MouseButton::Left,
          cx.listener(|this, _, window, cx| this.capture_tree_focus(window, cx)),
        )
        .can_drop(|value, _, _| {
          value
            .downcast_ref::<DragEntry>()
            .is_some_and(|entry| !drop_ignored(&entry.path, entry.is_directory, ""))
        })
        .drag_over::<DragEntry>(move |style, _, _, _| style.bg(hover))
        .on_drop::<DragEntry>(move |entry, window, cx| {
          let _ = drop_view.update(cx, |this, cx| this.drop_entry(entry, "", window, cx));
        })
        .context_menu(move |menu, _, _| {
          fill_menu(
            menu,
            &blank_menu_items(has_mark),
            String::new(),
            true,
            has_mark,
            blank_view.clone(),
          )
        })
        .child(
          Tree::new(&self.tree)
            .item(move |_, entry, entry_state, _, _| {
              let path = entry.item().id.as_str();
              if is_stub_id(path) {
                return div().h(px(0.0)).into_any_element();
              }
              let Some(row) = rows.iter().find(|row| row.path == path) else {
                return div().into_any_element();
              };
              let editing = if edit.as_ref().is_some_and(|edit| edit.path() == row.path) {
                edit_field.clone()
              } else {
                None
              };
              render_row(
                row,
                &RowPaint {
                  kind,
                  density,
                  palette,
                  selected: row.selected || entry_state.is_selected(),
                  has_mark,
                  editing,
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

  #[test]
  fn keyboard_selection_resyncs_when_the_tree_id_is_not_selected() {
    assert_eq!(
      keyboard_path_to_select(Some("b.rs"), &["a.rs".into()]).as_deref(),
      Some("b.rs")
    );
    assert_eq!(keyboard_path_to_select(Some("a.rs"), &["a.rs".into()]), None);
    assert_eq!(keyboard_path_to_select(None, &["a.rs".into()]), None);
    assert_eq!(keyboard_path_to_select(Some("src\u{2060}"), &["a.rs".into()]), None);
    assert_eq!(
      keyboard_path_to_select(Some("a.rs"), &["a.rs".into(), "b.rs".into()]),
      None
    );
    assert_eq!(
      keyboard_path_to_select(Some("b.rs"), &["a.rs".into(), "b.rs".into()]),
      None
    );
  }
}
