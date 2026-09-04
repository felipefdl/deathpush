use deathpush_core::config::layout::SidebarView;
use gpui_kit::*;

use crate::theme::{ActivePalette, hsla};

/// Two equal tabs, uppercase small bold; the active one at full opacity with a 2px accent underline.
pub fn render_sidebar(
  active: SidebarView,
  on_select: impl Fn(SidebarView, &mut Window, &mut App) + 'static + Clone,
  body: AnyElement,
  cx: &App,
) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let tab = |id: &'static str, label: &'static str, view: SidebarView| {
    let is_active = active == view;
    let on_select = on_select.clone();
    div()
      .id(id)
      .flex_1()
      .h(px(35.0))
      .flex()
      .items_center()
      .justify_center()
      .text_size(px(11.0))
      .font_weight(FontWeight::BOLD)
      .cursor_pointer()
      .border_b_2()
      .border_color(if is_active {
        hsla(palette.ring)
      } else {
        hsla(palette.border.with_alpha(0))
      })
      .opacity(if is_active { 1.0 } else { 0.5 })
      .child(label.to_uppercase())
      .on_click(move |_, window, cx| on_select(view, window, cx))
  };
  div()
    .size_full()
    .flex()
    .flex_col()
    .bg(hsla(palette.sidebar))
    .text_color(hsla(palette.sidebar_foreground))
    .child(
      div()
        .flex()
        .flex_shrink_0()
        .border_b_1()
        .border_color(hsla(palette.border))
        .child(tab("tab-changes", "Changes", SidebarView::Scm))
        .child(tab("tab-explorer", "Explorer", SidebarView::Explorer)),
    )
    .child(div().flex_1().min_h_0().child(body))
}
