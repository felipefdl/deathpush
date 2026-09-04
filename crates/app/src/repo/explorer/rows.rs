use deathpush_core::config::settings::TreeDensity;
use deathpush_core::theme::UiPalette;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::icons::{IconKind, icon_for, row_height};
use super::model::Row;
use super::view::ExplorerView;
use crate::repo::changes::rows::{status_color, status_letter};
use crate::theme::hsla;

pub struct RowPaint {
  pub kind: IconKind,
  pub density: TreeDensity,
  pub palette: UiPalette,
  pub selected: bool,
}

pub fn render_row(row: &Row, paint: &RowPaint, view: WeakEntity<ExplorerView>) -> AnyElement {
  let path = row.path.clone();
  let is_directory = row.is_directory;
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
  div()
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
    .when(row.ignored, |el| el.opacity(0.6))
    .on_mouse_down(MouseButton::Left, move |event, window, cx| {
      let _ = view.update(cx, |this, cx| {
        this.on_row_mouse_down(&path, is_directory, event, window, cx);
      });
    })
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
      el.child(
        svg()
          .path(path)
          .size(px(16.0))
          .flex_shrink_0()
          .text_color(hsla(paint.palette.muted_foreground)),
      )
    })
    .child(
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
    .into_any_element()
}
