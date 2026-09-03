use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

/// Temporary welcome screen. Task 8 replaces this.
pub struct WelcomeView {}

impl WelcomeView {
  pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
    Self {}
  }
}

impl Render for WelcomeView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .flex()
      .items_center()
      .justify_center()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .child("Welcome")
  }
}
