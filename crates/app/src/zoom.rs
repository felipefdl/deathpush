use deathpush_core::config::settings::{ZOOM_MAX, ZOOM_MIN, zoom_scale};
use gpui_kit::*;

use crate::config::AppConfig;

#[allow(dead_code)]
pub const BASE_REM: f32 = 16.0;

#[allow(dead_code)]
pub fn apply_zoom_to_window(level: i32, window: &mut Window) {
  window.set_rem_size(px(BASE_REM * zoom_scale(level)));
  window.refresh();
}

/// Persist the level and apply it to every open window.
#[allow(dead_code)]
pub fn set_zoom_level(level: i32, cx: &mut App) {
  let level = level.clamp(ZOOM_MIN, ZOOM_MAX);
  AppConfig::update(cx, |config| config.settings.ui.zoom_level = level);
  for handle in cx.windows() {
    let _ = handle.update(cx, |_, window, _| apply_zoom_to_window(level, window));
  }
}

#[allow(dead_code)]
pub fn current_level(cx: &App) -> i32 {
  AppConfig::get(cx).settings.ui.zoom_level
}

#[allow(dead_code)]
pub fn zoom_percent(level: i32) -> i32 {
  (zoom_scale(level) * 100.0).round() as i32
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn percent_matches_the_app_shell_spec() {
    assert_eq!(zoom_percent(0), 100);
    assert_eq!(zoom_percent(1), 120);
    assert_eq!(zoom_percent(-1), 83);
  }
}
