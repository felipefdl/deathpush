use gpui_kit::component::spinner::Spinner;
use gpui_kit::*;

use crate::theme::{ActivePalette, hsla};

/// Full-window dimmer with a spinner and `Opening repository...`.
pub fn render_opening(cx: &App) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  div()
    .absolute()
    .inset_0()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_3()
    .bg(hsla(palette.overlay))
    .text_color(hsla(palette.foreground))
    .child(Spinner::new())
    .child("Opening repository...")
}
