use deathpush_core::config::layout::MainView;
use gpui_kit::*;

use super::diff::DiffPanel;
use super::file_viewer::FileViewer;
use crate::theme::{ActivePalette, hsla};

/// The main panel body for a view. Changes shows the SCM diff; File shows the explorer viewer.
pub fn render_main_panel(
  view: MainView,
  diff: &Entity<DiffPanel>,
  file: &Entity<FileViewer>,
  cx: &App,
) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let body: AnyElement = match view {
    MainView::Changes => diff.clone().into_any_element(),
    MainView::File => file.clone().into_any_element(),
    MainView::History => div().size_full().into_any_element(),
    MainView::Settings => div().size_full().into_any_element(),
  };
  div()
    .size_full()
    .bg(hsla(palette.background))
    .text_color(hsla(palette.foreground))
    .child(body)
}
