use std::path::PathBuf;
use std::time::Duration;

use deathpush_core::config::recents::Recents;
use deathpush_core::config::settings::Settings;
use deathpush_core::config::windows::WindowsState;
use deathpush_core::config::{config_dir, read_json, write_json_atomic};
use gpui_kit::*;

const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

/// Settings, recents, and window bounds, loaded once and saved after a short quiet period.
pub struct AppConfig {
  pub settings: Settings,
  pub recents: Recents,
  pub windows: WindowsState,
  dir: PathBuf,
  revision: u64,
}

impl Global for AppConfig {}

impl AppConfig {
  pub fn init(cx: &mut App) {
    Self::init_at(config_dir(), cx);
  }

  pub fn init_at(dir: PathBuf, cx: &mut App) {
    let config = Self {
      settings: read_json(&dir.join("settings.json")),
      recents: read_json(&dir.join("recent-projects.json")),
      windows: read_json(&dir.join("windows.json")),
      dir,
      revision: 0,
    };
    cx.set_global(config);
  }

  pub fn get(cx: &App) -> &Self {
    cx.global::<Self>()
  }

  /// Mutate in place, then save once the changes go quiet for half a second.
  pub fn update(cx: &mut App, mutate: impl FnOnce(&mut Self)) {
    let revision = {
      let config = cx.global_mut::<Self>();
      mutate(config);
      config.revision += 1;
      config.revision
    };
    cx.spawn(async move |cx| {
      cx.background_executor().timer(SAVE_DEBOUNCE).await;
      cx.update(|cx| {
        let config = cx.global::<Self>();
        if config.revision == revision {
          config.save();
        }
      });
    })
    .detach();
  }

  pub fn save(&self) {
    for (name, result) in [
      (
        "settings.json",
        write_json_atomic(&self.dir.join("settings.json"), &self.settings),
      ),
      (
        "recent-projects.json",
        write_json_atomic(&self.dir.join("recent-projects.json"), &self.recents),
      ),
      (
        "windows.json",
        write_json_atomic(&self.dir.join("windows.json"), &self.windows),
      ),
    ] {
      if let Err(err) = result {
        tracing::warn!("could not save {name}: {err}");
      }
    }
  }

  #[allow(dead_code)]
  pub fn save_now(cx: &App) {
    cx.global::<Self>().save();
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  #[gpui_kit::test]
  fn update_saves_once_after_the_debounce(cx: &mut TestAppContext) {
    let dir = tempfile::TempDir::new().unwrap();
    let path = dir.path().to_path_buf();
    cx.update(|cx| AppConfig::init_at(path.clone(), cx));
    cx.update(|cx| AppConfig::update(cx, |config| config.settings.ui.zoom_level = 2));
    cx.update(|cx| AppConfig::update(cx, |config| config.settings.ui.zoom_level = 3));
    assert!(!path.join("settings.json").exists());
    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    let saved: Settings = read_json(&path.join("settings.json"));
    assert_eq!(saved.ui.zoom_level, 3);
  }

  #[gpui_kit::test]
  fn missing_files_load_defaults(cx: &mut TestAppContext) {
    let dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| AppConfig::init_at(dir.path().to_path_buf(), cx));
    cx.update(|cx| {
      let config = AppConfig::get(cx);
      assert_eq!(config.settings.ui.font_size, 13);
      assert!(config.recents.projects.is_empty());
    });
  }
}
