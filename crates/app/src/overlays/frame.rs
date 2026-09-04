use gpui_kit::*;

use crate::keymap::CONTEXT_DIALOG;
use crate::theme::{ActivePalette, hsla};

/// Transparent full-window backdrop; `on_backdrop` fires for clicks outside the panel.
pub fn backdrop(id: &'static str, on_backdrop: impl Fn(&mut Window, &mut App) + 'static, cx: &App) -> Stateful<Div> {
  let _ = cx;
  div()
    .id(id)
    .absolute()
    .inset_0()
    .flex()
    .justify_center()
    .items_start()
    .on_mouse_down(MouseButton::Left, move |_, window, cx| on_backdrop(window, cx))
}

/// The 440-wide (or `width`) panel, 60 from the top, on the sidebar background with the clone-dialog look.
pub fn dialog_frame(width: f32, title: &str, cx: &App) -> Div {
  let palette = cx.global::<ActivePalette>().0;
  div()
    .key_context(CONTEXT_DIALOG)
    .occlude()
    .mt(px(60.0))
    .w(px(width))
    .h_auto()
    .p(px(16.0))
    .bg(hsla(palette.sidebar))
    .border_1()
    .border_color(hsla(palette.border))
    .rounded_lg()
    .shadow_lg()
    .flex()
    .flex_col()
    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
    .child(
      div()
        .text_size(px(14.0))
        .font_weight(FontWeight::SEMIBOLD)
        .mb(px(12.0))
        .child(title.to_string()),
    )
}
