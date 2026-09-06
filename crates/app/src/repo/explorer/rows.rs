use deathpush_core::config::settings::TreeDensity;
use deathpush_core::theme::UiPalette;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::component::input::InputState;
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::icons::{IconKind, icon_for, render_icon, row_height};
use super::menus::ItemMenu;
use super::model::{Row, parent_path};
use super::view::ExplorerView;
use crate::repo::changes::rows::{status_color, status_letter};
use crate::theme::hsla;

pub struct RowPaint {
  pub kind: IconKind,
  pub density: TreeDensity,
  pub palette: UiPalette,
  pub selected: bool,
  pub has_mark: bool,
  pub editing: Option<Entity<InputState>>,
}

#[derive(Clone)]
pub struct DragEntry {
  pub path: String,
  pub is_directory: bool,
}

impl DragEntry {
  pub fn name(&self) -> &str {
    self.path.rsplit('/').next().unwrap_or(&self.path)
  }
}

pub struct DragPreview(pub String);

impl Render for DragPreview {
  fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<crate::theme::ActivePalette>().0;
    div()
      .px_2()
      .text_size(px(13.0))
      .text_color(hsla(palette.foreground))
      .bg(hsla(palette.list_hover))
      .child(self.0.clone())
  }
}

pub fn drop_ignored(source: &str, is_directory: bool, into: &str) -> bool {
  if parent_path(source) == into {
    return true;
  }
  is_directory && (into == source || into.strip_prefix(source).is_some_and(|rest| rest.starts_with('/')))
}

pub fn fill_menu(
  menu: PopupMenu,
  items: &[ItemMenu],
  path: String,
  is_directory: bool,
  has_mark: bool,
  view: WeakEntity<ExplorerView>,
) -> PopupMenu {
  let mut menu = menu.min_w(px(180.));
  for item in items {
    let item = *item;
    let view = view.clone();
    let path = path.clone();
    menu = menu.item(
      PopupMenuItem::new(item.label())
        .disabled(!item.enabled(is_directory, has_mark))
        .on_click(move |_, window, cx| {
          let _ = view.update(cx, |this, cx| this.on_item_menu(item, &path, is_directory, window, cx));
        }),
    );
  }
  menu
}

pub fn render_row(row: &Row, paint: &RowPaint, view: WeakEntity<ExplorerView>) -> AnyElement {
  let path = row.path.clone();
  let is_directory = row.is_directory;
  let editing = paint.editing.is_some();
  let chevron = if row.is_directory {
    if row.expanded {
      "icons/chevron-down.svg"
    } else {
      "icons/chevron-right.svg"
    }
  } else {
    ""
  };
  let icon = icon_for(paint.kind, &row.name, row.is_directory, row.expanded);
  let status = row.status.clone();
  let hover = hsla(paint.palette.list_hover);
  let menu_path = path.clone();
  let menu_view = view.clone();
  let has_mark = paint.has_mark;
  let drag = DragEntry {
    path: path.clone(),
    is_directory,
  };
  let drop_into = if is_directory { path.clone() } else { parent_path(&path) };
  let drop_into_style = drop_into.clone();
  let drop_view = view.clone();
  let mut row_el = div()
    .id(SharedString::from(row.path.clone()))
    .h(px(row_height(paint.density)))
    .flex_shrink_0()
    .flex()
    .items_center()
    .px_1()
    .pl(px(12.0 * row.depth as f32))
    .cursor_pointer()
    .when(paint.selected, |el| el.bg(hsla(paint.palette.list_active)))
    .when(!paint.selected, |el| {
      el.hover(|el| el.bg(hsla(paint.palette.list_hover)))
    })
    .when(row.ignored, |el| el.opacity(0.6));
  if !editing {
    let click_path = path.clone();
    let click_view = view.clone();
    row_el = row_el.on_mouse_down(MouseButton::Left, move |event, window, cx| {
      let _ = click_view.update(cx, |this, cx| {
        this.on_row_mouse_down(&click_path, is_directory, event, window, cx);
      });
    });
    row_el = row_el.on_drag(drag, |entry, _, _, cx| {
      cx.new(|_| DragPreview(entry.name().to_string()))
    });
  }
  row_el = row_el
    .can_drop(|value, _, _| value.downcast_ref::<DragEntry>().is_some())
    .drag_over::<DragEntry>(move |style, entry, _, _| {
      if drop_ignored(&entry.path, entry.is_directory, &drop_into_style) {
        style
      } else {
        style.bg(hover)
      }
    })
    .on_drop::<DragEntry>(move |entry, window, cx| {
      let _ = drop_view.update(cx, |this, cx| this.drop_entry(entry, &drop_into, window, cx));
    });
  row_el
    .child(
      div()
        .w(px(16.0))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .when(row.is_directory, |el| {
          el.child(
            svg()
              .path(chevron)
              .size(px(12.0))
              .text_color(hsla(paint.palette.muted_foreground)),
          )
        }),
    )
    .when_some(icon, |el, path| {
      el.child(render_icon(paint.kind, path, hsla(paint.palette.muted_foreground)))
    })
    .when_some(paint.editing.clone(), |el, field| {
      el.child(
        Input::new(&field)
          .small()
          .h(px(22.0))
          .w_full()
          .rounded_md()
          .bg(hsla(paint.palette.input)),
      )
    })
    .when(paint.editing.is_none(), |el| {
      el.child(
        div()
          .flex_1()
          .min_w_0()
          .overflow_hidden()
          .text_ellipsis()
          .text_size(px(13.0))
          .text_color(hsla(paint.palette.foreground))
          .pl(px(4.0))
          .child(row.name.clone()),
      )
    })
    .when_some(status.filter(|_| !row.is_directory), |el, status| {
      el.child(
        div()
          .w(px(16.0))
          .flex_shrink_0()
          .text_size(px(11.0))
          .text_color(hsla(status_color(status.clone(), &paint.palette)))
          .child(status_letter(status)),
      )
    })
    .context_menu(move |menu, _, _| {
      fill_menu(
        menu,
        &ItemMenu::ORDER,
        menu_path.clone(),
        is_directory,
        has_mark,
        menu_view.clone(),
      )
    })
    .into_any_element()
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn drop_onto_a_file_uses_the_parent_and_ignores_siblings() {
    let into = parent_path("src/b.rs");
    assert_eq!(into, "src");
    assert!(drop_ignored("src/a.rs", false, &into));
    assert!(!drop_ignored("a.rs", false, &into));
  }
}
