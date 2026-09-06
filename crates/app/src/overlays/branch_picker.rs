use std::sync::Arc;

use deathpush_core::session::types::{DEFAULT_REMOTE, Intent};
use deathpush_core::theme::UiPalette;
use deathpush_core::types::{BranchEntry, TagEntry};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::frame::backdrop;
use crate::actions::{Cancel, Confirm};
use crate::keymap::CONTEXT_BRANCH_PICKER;
use crate::repo::model::RepoModel;
use crate::repo::state::NetworkOp;
use crate::theme::{ActivePalette, hsla};

/// One branch in the picker list after filtering and ordering.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BranchRow {
  pub name: String,
  pub is_head: bool,
  pub is_remote: bool,
  pub ahead: usize,
  pub behind: usize,
}

fn matches_query(name: &str, needle: &str) -> bool {
  needle.is_empty() || name.to_lowercase().contains(needle)
}

fn kind_rank(is_head: bool, is_remote: bool) -> u8 {
  if is_head {
    0
  } else if is_remote {
    2
  } else {
    1
  }
}

/// Current first, then local, then remote, each by name; case-insensitive substring filter.
pub fn branch_rows(branches: &[BranchEntry], query: &str) -> Vec<BranchRow> {
  let needle = query.trim().to_lowercase();
  let mut rows: Vec<BranchRow> = branches
    .iter()
    .filter(|branch| matches_query(&branch.name, &needle))
    .map(|branch| BranchRow {
      name: branch.name.clone(),
      is_head: branch.is_head,
      is_remote: branch.is_remote,
      ahead: branch.ahead,
      behind: branch.behind,
    })
    .collect();
  rows.sort_by(|a, b| {
    kind_rank(a.is_head, a.is_remote)
      .cmp(&kind_rank(b.is_head, b.is_remote))
      .then_with(|| a.name.cmp(&b.name))
  });
  rows
}

/// Case-insensitive substring filter over tag names, preserving list order.
pub fn tag_rows(tags: &[TagEntry], query: &str) -> Vec<TagEntry> {
  let needle = query.trim().to_lowercase();
  tags
    .iter()
    .filter(|tag| matches_query(&tag.name, &needle))
    .cloned()
    .collect()
}

/// Some(trimmed) when the trimmed query is non-empty and no listed branch (or tag, for tags) equals it case-sensitively.
pub fn create_candidate(names: &[&str], query: &str) -> Option<String> {
  let trimmed = query.trim();
  if trimmed.is_empty() || names.contains(&trimmed) {
    None
  } else {
    Some(trimmed.to_string())
  }
}

/// Behind first, then ahead. A zero count is omitted.
pub fn ahead_behind_badges(ahead: usize, behind: usize) -> Vec<String> {
  let mut badges = Vec::new();
  if behind > 0 {
    badges.push(format!("{behind}↓"));
  }
  if ahead > 0 {
    badges.push(format!("{ahead}↑"));
  }
  badges
}

/// Context-menu labels in spec order. `{name}` is replaced when rendering rebase.
pub const BRANCH_MENU: [&str; 7] = [
  "Checkout",
  "Copy Branch Name",
  "Merge into Current Branch",
  "Rebase onto {name}",
  "Rename Branch...",
  "Delete Branch",
  "Delete Remote Branch",
];

/// Current: checkout, copy, and rename. Origin remotes add remote delete. Other remotes omit it. Local non-current: all but remote delete.
pub fn branch_menu_items(row: &BranchRow) -> Vec<&'static str> {
  if row.is_head {
    vec![BRANCH_MENU[0], BRANCH_MENU[1], BRANCH_MENU[4]]
  } else if row.is_remote {
    if origin_branch_name(&row.name).is_some() {
      vec![BRANCH_MENU[0], BRANCH_MENU[1], BRANCH_MENU[6]]
    } else {
      vec![BRANCH_MENU[0], BRANCH_MENU[1]]
    }
  } else {
    BRANCH_MENU[..6].to_vec()
  }
}

fn remote_short_name(name: &str) -> &str {
  match name.split_once('/') {
    Some((_, rest)) if !rest.is_empty() => rest,
    _ => name,
  }
}

/// Checkout a local row, or create/switch the tracking branch for a remote row.
pub fn checkout_intent(row: &BranchRow, local_names: &[String]) -> Intent {
  if !row.is_remote {
    return Intent::CheckoutBranch { name: row.name.clone() };
  }
  let short = remote_short_name(&row.name).to_string();
  if local_names.iter().any(|name| name == &short) {
    Intent::CheckoutBranch { name: short }
  } else {
    Intent::CreateBranch {
      name: short,
      from: Some(row.name.clone()),
    }
  }
}

fn origin_branch_name(name: &str) -> Option<&str> {
  name
    .strip_prefix(DEFAULT_REMOTE)
    .and_then(|rest| rest.strip_prefix('/'))
    .filter(|short| !short.is_empty())
}

fn remote_delete_message(kind: &str, name: &str) -> String {
  format!("Are you sure you want to delete remote {kind} \"{name}\"?\n\nThis cannot be undone.")
}

fn prompt_accepted(choice: usize) -> bool {
  choice == 0
}

fn remote_delete_intent(listed_name: &str, accepted: bool) -> Option<Intent> {
  if !accepted {
    return None;
  }
  Some(Intent::DeleteRemoteBranch {
    name: origin_branch_name(listed_name)?.to_string(),
  })
}

fn remote_tag_intent(name: String, accepted: bool) -> Option<Intent> {
  accepted.then_some(Intent::DeleteRemoteTag { name })
}

fn delete_local_branch_intent(name: String) -> Intent {
  Intent::DeleteBranch {
    name,
    force: false,
    confirmed: false,
  }
}

fn rename_decision(old_name: &str, new_name: &str) -> Option<String> {
  let new_name = new_name.trim();
  if new_name.is_empty() || new_name == old_name {
    None
  } else {
    Some(new_name.to_string())
  }
}

fn menu_icon(item: &str) -> &'static str {
  match item {
    "Checkout" => "icons/git-branch.svg",
    "Copy Branch Name" => "icons/copy.svg",
    "Merge into Current Branch" => "icons/git-commit-horizontal.svg",
    "Rebase onto {name}" => "icons/arrow-left-right.svg",
    "Rename Branch..." => "icons/pencil.svg",
    "Delete Branch" => "icons/trash.svg",
    "Delete Remote Branch" => "icons/cloud.svg",
    _ => "icons/git-branch.svg",
  }
}

/// Close the overlay.
pub enum BranchPickerEvent {
  Close,
}

#[derive(Clone, Default)]
struct Derived {
  branches: Vec<BranchRow>,
  tags: Vec<TagEntry>,
  create_branch: Option<String>,
  create_tag: Option<String>,
  query_empty: bool,
}

impl Derived {
  fn build(branches: &[BranchEntry], tags: &[TagEntry], query: &str) -> Self {
    let listed = branch_rows(branches, query);
    let listed_tags = tag_rows(tags, query);
    let branch_names: Vec<&str> = branches.iter().map(|branch| branch.name.as_str()).collect();
    let tag_names: Vec<&str> = tags.iter().map(|tag| tag.name.as_str()).collect();
    Self {
      branches: listed,
      tags: listed_tags,
      create_branch: create_candidate(&branch_names, query),
      create_tag: create_candidate(&tag_names, query),
      query_empty: query.trim().is_empty(),
    }
  }
}

#[derive(Clone, Copy)]
enum ListItem {
  Branch(usize),
  CreateBranch,
  TagsHeader,
  NoTags,
  Tag(usize),
  CreateTag,
}

/// Branch and tag switcher overlay opened from the status bar.
pub struct BranchPicker {
  model: Entity<RepoModel>,
  search: Entity<InputState>,
  tags_open: bool,
  rename: Option<String>,
  rename_field: Option<Entity<InputState>>,
  rename_sub: Option<Subscription>,
  derived: Arc<Derived>,
}

impl EventEmitter<BranchPickerEvent> for BranchPicker {}

impl BranchPicker {
  pub fn new(model: Entity<RepoModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
    cx.observe(&model, |this, _, cx| {
      this.refresh(cx);
      cx.notify();
    })
    .detach();
    let search = cx.new(|cx| InputState::new(window, cx).placeholder("Switch to branch..."));
    search.update(cx, |state, cx| state.focus(window, cx));
    cx.subscribe(&search, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        this.refresh(cx);
        cx.notify();
      }
    })
    .detach();
    let mut this = Self {
      model,
      search,
      tags_open: false,
      rename: None,
      rename_field: None,
      rename_sub: None,
      derived: Arc::new(Derived::default()),
    };
    this.refresh(cx);
    this
  }

  fn refresh(&mut self, cx: &App) {
    let query = self.search.read(cx).value().to_string();
    let state = self.model.read(cx).state();
    self.derived = Arc::new(Derived::build(&state.branches, &state.tags, &query));
  }

  fn list_items(&self) -> Vec<ListItem> {
    let derived = self.derived.as_ref();
    let mut items = Vec::with_capacity(derived.branches.len() + derived.tags.len() + 4);
    items.extend((0..derived.branches.len()).map(ListItem::Branch));
    if derived.create_branch.is_some() {
      items.push(ListItem::CreateBranch);
    }
    items.push(ListItem::TagsHeader);
    if self.tags_open {
      if derived.tags.is_empty() && derived.query_empty {
        items.push(ListItem::NoTags);
      }
      items.extend((0..derived.tags.len()).map(ListItem::Tag));
      if derived.create_tag.is_some() {
        items.push(ListItem::CreateTag);
      }
    }
    items
  }

  pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
    self.search.update(cx, |state, cx| state.focus(window, cx));
  }

  fn close(&mut self, cx: &mut Context<Self>) {
    cx.emit(BranchPickerEvent::Close);
  }

  fn send(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }

  fn send_network(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| {
      model.dispatch_network(NetworkOp::Push, intent, window, cx)
    });
  }

  fn send_and_close(&mut self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.send(intent, window, cx);
    self.close(cx);
  }

  fn confirm(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.rename.is_some() || self.rename_field.is_some() {
      self.save_rename(window, cx);
      return;
    }
    if let Some(row) = self.derived.branches.first().cloned() {
      self.checkout(&row, window, cx);
    }
  }

  fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.rename.is_some() || self.rename_field.is_some() {
      self.cancel_rename(window, cx);
      return;
    }
    self.close(cx);
  }

  fn checkout(&mut self, row: &BranchRow, window: &mut Window, cx: &mut Context<Self>) {
    let locals: Vec<String> = self
      .model
      .read(cx)
      .state()
      .branches
      .iter()
      .filter(|branch| !branch.is_remote)
      .map(|branch| branch.name.clone())
      .collect();
    self.send_and_close(checkout_intent(row, &locals), window, cx);
  }

  fn create_branch(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.send_and_close(Intent::CreateBranch { name, from: None }, window, cx);
  }

  fn create_tag(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.send(
      Intent::CreateTag {
        name,
        message: None,
        commit: None,
      },
      window,
      cx,
    );
    self.search.update(cx, |state, cx| {
      state.set_value("", window, cx);
      state.focus(window, cx);
    });
    self.refresh(cx);
    cx.notify();
  }

  fn start_rename(&mut self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.cancel_rename(window, cx);
    let field = cx.new(|cx| {
      let mut state = InputState::new(window, cx);
      state.set_value(name.clone(), window, cx);
      state.select_all(window, cx);
      state
    });
    self.rename_sub = Some(cx.subscribe_in(&field, window, |this, _, event, window, cx| {
      if matches!(event, InputEvent::Blur) {
        this.save_rename(window, cx);
      }
    }));
    self.rename = Some(name);
    self.rename_field = Some(field.clone());
    cx.defer_in(window, |this, window, cx| {
      if let Some(field) = this.rename_field.clone() {
        field.update(cx, |state, cx| state.focus(window, cx));
      }
    });
    cx.notify();
  }

  fn cancel_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if self.rename.is_none() && self.rename_field.is_none() {
      return;
    }
    self.rename = None;
    self.rename_field = None;
    self.rename_sub = None;
    self.search.update(cx, |state, cx| state.focus(window, cx));
    cx.notify();
  }

  fn save_rename(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let Some(old_name) = self.rename.take() else {
      self.rename_field = None;
      self.rename_sub = None;
      return;
    };
    let new_name = self
      .rename_field
      .as_ref()
      .map(|field| field.read(cx).value().to_string())
      .unwrap_or_default();
    self.rename_field = None;
    self.rename_sub = None;
    if let Some(new_name) = rename_decision(&old_name, &new_name) {
      self.send(Intent::RenameBranch { old_name, new_name }, window, cx);
    }
    self.search.update(cx, |state, cx| state.focus(window, cx));
    cx.notify();
  }

  fn on_menu(&mut self, item: &'static str, row: &BranchRow, window: &mut Window, cx: &mut Context<Self>) {
    match item {
      "Checkout" => self.checkout(row, window, cx),
      "Copy Branch Name" => cx.write_to_clipboard(ClipboardItem::new_string(row.name.clone())),
      "Merge into Current Branch" => self.send_and_close(Intent::MergeBranch { name: row.name.clone() }, window, cx),
      "Rebase onto {name}" => self.send_and_close(Intent::RebaseBranch { name: row.name.clone() }, window, cx),
      "Rename Branch..." => self.start_rename(row.name.clone(), window, cx),
      "Delete Branch" => self.send(delete_local_branch_intent(row.name.clone()), window, cx),
      "Delete Remote Branch" => self.delete_remote_branch(row.name.clone(), window, cx),
      _ => {}
    }
  }

  fn delete_remote_branch(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    if origin_branch_name(&name).is_none() {
      return;
    }
    self.prompt_remote_delete(
      "Delete Remote Branch",
      remote_delete_message("branch", &name),
      name,
      true,
      window,
      cx,
    );
  }

  fn delete_remote_tag(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.prompt_remote_delete(
      "Delete Remote Tag",
      remote_delete_message("tag", &name),
      name,
      false,
      window,
      cx,
    );
  }

  fn prompt_remote_delete(
    &self,
    title: &'static str,
    message: String,
    listed: String,
    branch: bool,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let answer = window.prompt(PromptLevel::Warning, title, Some(&message), &["Delete", "Cancel"], cx);
    cx.spawn_in(window, async move |this, cx| {
      let Ok(choice) = answer.await else {
        return;
      };
      let _ = this.update_in(cx, |this, window, cx| {
        let intent = if branch {
          remote_delete_intent(&listed, prompt_accepted(choice))
        } else {
          remote_tag_intent(listed, prompt_accepted(choice))
        };
        if let Some(intent) = intent {
          this.send_network(intent, window, cx);
        }
      });
    })
    .detach();
  }

  fn push_tag(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.send_network(Intent::PushTag { name }, window, cx);
  }

  fn delete_tag(&self, name: String, window: &mut Window, cx: &mut Context<Self>) {
    self.send(Intent::DeleteTag { name, confirmed: false }, window, cx);
  }
}

impl Render for BranchPicker {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let derived = self.derived.clone();
    let items = Arc::new(self.list_items());
    let count = items.len();
    let view = cx.weak_entity();
    let renaming = self.rename.clone();
    let rename_field = self.rename_field.clone();
    let tags_open = self.tags_open;
    let list = uniform_list("branch-picker-list", count, move |range, _, _| {
      range
        .filter_map(|index| {
          let item = *items.get(index)?;
          Some(render_list_item(
            item,
            &derived,
            tags_open,
            renaming.as_deref(),
            rename_field.clone(),
            view.clone(),
            palette,
          ))
        })
        .collect()
    });

    backdrop("branch-picker-backdrop", |_, _| {}, cx)
      .on_mouse_down(MouseButton::Left, cx.listener(|this, _, _, cx| this.close(cx)))
      .child(
        div()
          .key_context(CONTEXT_BRANCH_PICKER)
          .occlude()
          .mt(px(60.0))
          .w(px(400.0))
          .h(px((32.0 + count as f32 * 26.0 + 2.0).min(300.0)))
          .flex()
          .flex_col()
          .overflow_hidden()
          .bg(hsla(palette.sidebar))
          .border_1()
          .border_color(hsla(palette.border))
          .rounded_lg()
          .shadow_lg()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .on_action(cx.listener(|this, _: &Confirm, window, cx| this.confirm(window, cx)))
          .on_action(cx.listener(|this, _: &Cancel, window, cx| this.cancel(window, cx)))
          .child(
            div()
              .h(px(32.0))
              .flex_shrink_0()
              .flex()
              .items_center()
              .px_2()
              .bg(hsla(palette.input))
              .border_b_1()
              .border_color(hsla(palette.border))
              .child(Input::new(&self.search).appearance(false).h(px(32.0)).w_full()),
          )
          .child(list.flex_1().min_h_0()),
      )
  }
}

fn render_list_item(
  item: ListItem,
  derived: &Derived,
  tags_open: bool,
  renaming: Option<&str>,
  rename_field: Option<Entity<InputState>>,
  view: WeakEntity<BranchPicker>,
  palette: UiPalette,
) -> AnyElement {
  match item {
    ListItem::Branch(index) => {
      let row = &derived.branches[index];
      let editing = renaming == Some(row.name.as_str());
      render_branch_row(row, editing, rename_field, view, palette)
    }
    ListItem::CreateBranch => {
      let name = derived.create_branch.clone().unwrap_or_default();
      render_create_row(
        "branch-picker-create-branch",
        format!("Create branch: {name}"),
        name,
        view,
        palette,
        true,
      )
    }
    ListItem::TagsHeader => render_tags_header(derived.tags.len(), tags_open, view, palette),
    ListItem::NoTags => div()
      .h(px(26.0))
      .flex_shrink_0()
      .flex()
      .items_center()
      .px_2()
      .text_size(px(13.0))
      .text_color(hsla(palette.muted_foreground))
      .child("No tags")
      .into_any_element(),
    ListItem::Tag(index) => render_tag_row(&derived.tags[index], view, palette),
    ListItem::CreateTag => {
      let name = derived.create_tag.clone().unwrap_or_default();
      render_create_row(
        "branch-picker-create-tag",
        format!("Create tag: {name}"),
        name,
        view,
        palette,
        false,
      )
    }
  }
}

fn render_branch_row(
  row: &BranchRow,
  editing: bool,
  rename_field: Option<Entity<InputState>>,
  view: WeakEntity<BranchPicker>,
  palette: UiPalette,
) -> AnyElement {
  let icon = if row.is_head {
    "icons/check.svg"
  } else if row.is_remote {
    "icons/cloud.svg"
  } else {
    "icons/git-branch.svg"
  };
  let badges = if row.is_remote {
    Vec::new()
  } else {
    ahead_behind_badges(row.ahead, row.behind)
  };
  let name = row.name.clone();
  let click_row = row.clone();
  let click_view = view.clone();
  let menu_row = row.clone();
  let menu_view = view;
  let mut el = div()
    .id(SharedString::from(format!("branch-picker-row-{name}")))
    .h(px(26.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .hover(|el| el.bg(hsla(palette.list_hover)))
    .child(
      svg()
        .path(if editing { "icons/pencil.svg" } else { icon })
        .size(px(14.0))
        .text_color(hsla(palette.muted_foreground)),
    );
  if editing {
    if let Some(field) = rename_field {
      el = el.child(
        Input::new(&field)
          .small()
          .h(px(22.0))
          .w_full()
          .rounded_md()
          .bg(hsla(palette.input)),
      );
    }
    return el.into_any_element();
  }
  el = el
    .on_click(move |_, window, cx| {
      let _ = click_view.update(cx, |this, cx| this.checkout(&click_row, window, cx));
    })
    .child(
      div()
        .min_w_0()
        .flex_1()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(13.0))
        .text_color(hsla(palette.foreground))
        .child(row.name.clone()),
    );
  for badge in badges {
    el = el.child(
      div()
        .flex_shrink_0()
        .text_size(px(11.0))
        .opacity(0.7)
        .text_color(hsla(palette.muted_foreground))
        .child(badge),
    );
  }
  el.context_menu(move |menu, _, _| fill_branch_menu(menu, menu_row.clone(), menu_view.clone()))
    .into_any_element()
}

fn fill_branch_menu(menu: PopupMenu, row: BranchRow, view: WeakEntity<BranchPicker>) -> PopupMenu {
  let mut menu = menu.min_w(px(180.));
  for item in branch_menu_items(&row) {
    let label = if item == BRANCH_MENU[3] {
      format!("Rebase onto {}", row.name)
    } else {
      item.to_string()
    };
    let view = view.clone();
    let row = row.clone();
    menu = menu.item(
      PopupMenuItem::new(label)
        .icon(Icon::empty().path(menu_icon(item)))
        .on_click(move |_, window, cx| {
          let _ = view.update(cx, |this, cx| this.on_menu(item, &row, window, cx));
        }),
    );
  }
  menu
}

fn render_create_row(
  id: &'static str,
  label: String,
  name: String,
  view: WeakEntity<BranchPicker>,
  palette: UiPalette,
  is_branch: bool,
) -> AnyElement {
  div()
    .id(id)
    .h(px(26.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .hover(|el| el.bg(hsla(palette.list_hover)))
    .on_click(move |_, window, cx| {
      let _ = view.update(cx, |this, cx| {
        if is_branch {
          this.create_branch(name.clone(), window, cx);
        } else {
          this.create_tag(name.clone(), window, cx);
        }
      });
    })
    .child(
      svg()
        .path("icons/plus.svg")
        .size(px(14.0))
        .text_color(hsla(palette.git_added)),
    )
    .child(
      div()
        .min_w_0()
        .flex_1()
        .overflow_hidden()
        .text_ellipsis()
        .text_size(px(13.0))
        .text_color(hsla(palette.git_added))
        .child(label),
    )
    .into_any_element()
}

fn render_tags_header(count: usize, open: bool, view: WeakEntity<BranchPicker>, palette: UiPalette) -> AnyElement {
  div()
    .id("branch-picker-tags-header")
    .h(px(26.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .cursor_pointer()
    .border_t_1()
    .border_color(hsla(palette.border))
    .hover(|el| el.bg(hsla(palette.list_hover)))
    .on_click(move |_, _, cx| {
      let _ = view.update(cx, |this, cx| {
        this.tags_open = !this.tags_open;
        cx.notify();
      });
    })
    .child(
      svg()
        .path(if open {
          "icons/chevron-down.svg"
        } else {
          "icons/chevron-right.svg"
        })
        .size(px(14.0))
        .text_color(hsla(palette.muted_foreground)),
    )
    .child(
      div()
        .text_size(px(12.0))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(hsla(palette.muted_foreground))
        .child(format!("Tags ({count})").to_uppercase()),
    )
    .into_any_element()
}

fn hover_tool(id: SharedString, path: &'static str, tooltip: &'static str, group: SharedString) -> Button {
  Button::new(id)
    .ghost()
    .xsmall()
    .w(px(22.0))
    .h(px(22.0))
    .icon(Icon::empty().path(path))
    .tooltip(tooltip)
    .invisible()
    .group_hover(group, |style| style.visible())
}

fn render_tag_row(tag: &TagEntry, view: WeakEntity<BranchPicker>, palette: UiPalette) -> AnyElement {
  let hover = SharedString::from(format!("tag-row-{}", tag.name));
  let icon = if tag.is_annotated {
    "icons/bookmark.svg"
  } else {
    "icons/tag.svg"
  };
  let message = tag.message.clone();
  let push_name = tag.name.clone();
  let push_view = view.clone();
  let remote_name = tag.name.clone();
  let remote_view = view.clone();
  let delete_name = tag.name.clone();
  let delete_view = view;
  div()
    .id(SharedString::from(format!("branch-picker-tag-{}", tag.name)))
    .group(hover.clone())
    .h(px(26.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .hover(|el| el.bg(hsla(palette.list_hover)))
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
        .child(tag.name.clone()),
    )
    .when_some(message.filter(|text| !text.is_empty()), |el, text| {
      el.child(
        div()
          .max_w(px(120.0))
          .overflow_hidden()
          .text_ellipsis()
          .text_size(px(11.0))
          .text_color(hsla(palette.muted_foreground))
          .group_hover(hover.clone(), |style| style.invisible())
          .child(text),
      )
    })
    .child(
      hover_tool(
        SharedString::from(format!("push-tag-{}", tag.name)),
        "icons/cloud-upload.svg",
        "Push Tag",
        hover.clone(),
      )
      .on_click(move |_, window, cx| {
        cx.stop_propagation();
        let _ = push_view.update(cx, |this, cx| this.push_tag(push_name.clone(), window, cx));
      }),
    )
    .child(
      hover_tool(
        SharedString::from(format!("delete-remote-tag-{}", tag.name)),
        "icons/cloud.svg",
        "Delete Remote Tag",
        hover.clone(),
      )
      .on_click(move |_, window, cx| {
        cx.stop_propagation();
        let _ = remote_view.update(cx, |this, cx| this.delete_remote_tag(remote_name.clone(), window, cx));
      }),
    )
    .child(
      hover_tool(
        SharedString::from(format!("delete-tag-{}", tag.name)),
        "icons/trash.svg",
        "Delete Tag",
        hover,
      )
      .on_click(move |_, window, cx| {
        cx.stop_propagation();
        let _ = delete_view.update(cx, |this, cx| this.delete_tag(delete_name.clone(), window, cx));
      }),
    )
    .into_any_element()
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::session::types::Intent;

  fn branch(name: &str, is_head: bool, is_remote: bool, ahead: usize, behind: usize) -> BranchEntry {
    BranchEntry {
      name: name.into(),
      is_head,
      is_remote,
      upstream: None,
      ahead,
      behind,
    }
  }

  fn row(name: &str, is_head: bool, is_remote: bool) -> BranchRow {
    BranchRow {
      name: name.into(),
      is_head,
      is_remote,
      ahead: 0,
      behind: 0,
    }
  }

  #[test]
  fn branch_rows_order_and_filter() {
    let branches = vec![
      branch("zeta", false, false, 0, 0),
      branch("origin/main", false, true, 0, 0),
      branch("alpha", false, false, 1, 2),
      branch("zzz", true, false, 0, 1),
      branch("origin/alpha", false, true, 0, 0),
    ];
    let rows = branch_rows(&branches, "");
    let names: Vec<&str> = rows.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["zzz", "alpha", "zeta", "origin/alpha", "origin/main"]);
    assert!(rows[0].is_head);
    assert_eq!((rows[1].ahead, rows[1].behind), (1, 2));

    let filtered = branch_rows(&branches, "ALP");
    let names: Vec<&str> = filtered.iter().map(|row| row.name.as_str()).collect();
    assert_eq!(names, ["alpha", "origin/alpha"]);
  }

  #[test]
  fn create_candidate_rules() {
    let names = ["main", "feat"];
    assert_eq!(create_candidate(&names, ""), None);
    assert_eq!(create_candidate(&names, "   "), None);
    assert_eq!(create_candidate(&names, "main"), None);
    assert_eq!(create_candidate(&names, "Main").as_deref(), Some("Main"));
    assert_eq!(create_candidate(&names, "  topic  ").as_deref(), Some("topic"));
  }

  #[test]
  fn badges_order_and_hide_zero() {
    assert!(ahead_behind_badges(0, 0).is_empty());
    assert_eq!(ahead_behind_badges(2, 0), ["2↑"]);
    assert_eq!(ahead_behind_badges(0, 3), ["3↓"]);
    assert_eq!(ahead_behind_badges(2, 1), ["1↓", "2↑"]);
  }

  #[test]
  fn menu_items_per_row_kind() {
    assert_eq!(
      branch_menu_items(&row("main", true, false)),
      ["Checkout", "Copy Branch Name", "Rename Branch..."]
    );
    assert_eq!(
      branch_menu_items(&row("feat", false, false)),
      [
        "Checkout",
        "Copy Branch Name",
        "Merge into Current Branch",
        "Rebase onto {name}",
        "Rename Branch...",
        "Delete Branch",
      ]
    );
    assert_eq!(
      branch_menu_items(&row("origin/main", false, true)),
      ["Checkout", "Copy Branch Name", "Delete Remote Branch"]
    );
    assert_eq!(
      branch_menu_items(&row("upstream/topic", false, true)),
      ["Checkout", "Copy Branch Name"]
    );
  }

  #[test]
  fn remote_checkout_creates_or_switches_tracking_branch() {
    let remote = row("origin/feat", false, true);
    assert_eq!(
      checkout_intent(&remote, &["main".into()]),
      Intent::CreateBranch {
        name: "feat".into(),
        from: Some("origin/feat".into()),
      }
    );
    assert_eq!(
      checkout_intent(&remote, &["feat".into()]),
      Intent::CheckoutBranch { name: "feat".into() }
    );
    assert_eq!(
      checkout_intent(&row("main", true, false), &["main".into()]),
      Intent::CheckoutBranch { name: "main".into() }
    );
  }

  #[test]
  fn origin_remote_name_handling() {
    assert_eq!(origin_branch_name("origin/main"), Some("main"));
    assert_eq!(origin_branch_name("origin/feat/x"), Some("feat/x"));
    assert_eq!(origin_branch_name("upstream/topic"), None);
    assert_eq!(origin_branch_name("main"), None);
    assert_eq!(origin_branch_name("origin"), None);
    assert_eq!(origin_branch_name("origin/"), None);
  }

  #[test]
  fn remote_confirmation_copy_matches_spec() {
    assert_eq!(
      remote_delete_message("branch", "origin/main"),
      "Are you sure you want to delete remote branch \"origin/main\"?\n\nThis cannot be undone."
    );
    assert_eq!(
      remote_delete_message("tag", "v1"),
      "Are you sure you want to delete remote tag \"v1\"?\n\nThis cannot be undone."
    );
  }

  #[test]
  fn cancel_produces_no_remote_delete_intent() {
    assert!(!prompt_accepted(1));
    assert_eq!(remote_delete_intent("origin/main", false), None);
    assert_eq!(remote_delete_intent("upstream/topic", true), None);
    assert_eq!(
      remote_delete_intent("origin/main", true),
      Some(Intent::DeleteRemoteBranch { name: "main".into() })
    );
  }

  #[test]
  fn local_delete_is_not_forced() {
    assert_eq!(
      delete_local_branch_intent("feat".into()),
      Intent::DeleteBranch {
        name: "feat".into(),
        force: false,
        confirmed: false,
      }
    );
  }

  #[test]
  fn empty_or_unchanged_rename_is_suppressed() {
    assert_eq!(rename_decision("feat", ""), None);
    assert_eq!(rename_decision("feat", "   "), None);
    assert_eq!(rename_decision("feat", "feat"), None);
    assert_eq!(rename_decision("feat", "  feat  "), None);
    assert_eq!(rename_decision("feat", " topic ").as_deref(), Some("topic"));
  }
}
