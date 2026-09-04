#![allow(dead_code)]

use deathpush_core::config::layout::MainView;
use gpui_kit::*;

use crate::theme::{ActivePalette, hsla};

/// The main panel body for a view. Changes shows the SCM diff empty state; the other views are slots for later plans.
pub fn render_main_panel(view: MainView, cx: &App) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let empty = |text: &'static str| {
    div()
      .size_full()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_3()
      .child(
        svg()
          .path("brand/deathpush.svg")
          .size(px(80.0))
          .text_color(hsla(palette.mark))
          .opacity(0.07),
      )
      .child(
        div()
          .text_size(px(13.0))
          .text_color(hsla(palette.foreground))
          .opacity(0.4)
          .child(text),
      )
  };
  let body: AnyElement = match view {
    MainView::Changes => empty("Select a file to view changes").into_any_element(),
    MainView::File => empty("Select a file to view").into_any_element(),
    MainView::History => div().size_full().into_any_element(),
    MainView::Settings => div().size_full().into_any_element(),
  };
  div()
    .size_full()
    .bg(hsla(palette.background))
    .text_color(hsla(palette.foreground))
    .child(body)
}
