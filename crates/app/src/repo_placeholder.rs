use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

/// Stands in for the repository chrome until the next plan. Shows the title and the status-event count.
pub struct RepoPlaceholder {
  pub title: SharedString,
  pub status_events: usize,
}

impl Render for RepoPlaceholder {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    div()
      .size_full()
      .flex()
      .flex_col()
      .items_center()
      .justify_center()
      .gap_2()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .child(self.title.clone())
      .child(
        div()
          .text_color(cx.theme().muted_foreground)
          .child(format!("status events: {}", self.status_events)),
      )
  }
}
