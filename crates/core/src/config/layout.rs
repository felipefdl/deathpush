use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::store::{read_json, write_json_atomic};
use crate::content_hash::sha256_utf8;
use crate::error::Result;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum MainView {
  #[default]
  Changes,
  History,
  Settings,
  File,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SidebarView {
  #[default]
  Scm,
  Explorer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum PanelTab {
  #[default]
  Terminal,
  GitOutput,
}

/// The per-project shell layout from docs/specs/app-shell.md, Persistence.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectLayout {
  pub sidebar_width: f32,
  pub terminal_visible: bool,
  pub terminal_height: f32,
  pub main_view: MainView,
  pub sidebar_view: SidebarView,
  pub panel_tab: PanelTab,
  pub collapsed_panes: Vec<String>,
  pub terminal_maximized: bool,
  pub history_list_width: f32,
}

impl Default for ProjectLayout {
  fn default() -> Self {
    Self {
      sidebar_width: 300.0,
      terminal_visible: true,
      terminal_height: 250.0,
      main_view: MainView::Changes,
      sidebar_view: SidebarView::Scm,
      panel_tab: PanelTab::Terminal,
      collapsed_panes: Vec::new(),
      terminal_maximized: false,
      history_list_width: 300.0,
    }
  }
}

impl ProjectLayout {
  /// Applied on load: transient views reset to Changes, sizes clamp to the spec ranges,
  /// and Always Open Terminal on Start forces the terminal visible.
  pub fn sanitized(mut self, always_open_terminal: bool) -> Self {
    if !matches!(self.main_view, MainView::Changes | MainView::History) {
      self.main_view = MainView::Changes;
    }
    self.sidebar_width = self.sidebar_width.clamp(200.0, 600.0);
    self.terminal_height = self.terminal_height.clamp(100.0, 600.0);
    self.history_list_width = self.history_list_width.clamp(200.0, 600.0);
    if always_open_terminal {
      self.terminal_visible = true;
    }
    self
  }
}

/// `<config dir>/projects/<first 16 hex of sha256(root)>.json`.
pub fn layout_path(config_dir: &Path, root: &str) -> PathBuf {
  let hash = sha256_utf8(root);
  config_dir.join("projects").join(format!("{}.json", &hash[..16]))
}

pub fn load_layout(config_dir: &Path, root: &str, always_open_terminal: bool) -> ProjectLayout {
  read_json::<ProjectLayout>(&layout_path(config_dir, root)).sanitized(always_open_terminal)
}

pub fn save_layout(config_dir: &Path, root: &str, layout: &ProjectLayout) -> Result<()> {
  write_json_atomic(&layout_path(config_dir, root), layout)
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_match_the_app_shell_spec() {
    let layout = ProjectLayout::default();
    assert_eq!(layout.sidebar_width, 300.0);
    assert_eq!(layout.terminal_height, 250.0);
    assert!(layout.terminal_visible);
    assert_eq!(layout.history_list_width, 300.0);
  }

  #[test]
  fn sanitized_resets_transient_views_and_clamps() {
    let layout = ProjectLayout {
      main_view: MainView::Settings,
      sidebar_width: 50.0,
      terminal_height: 9000.0,
      terminal_visible: false,
      ..Default::default()
    }
    .sanitized(true);
    assert_eq!(layout.main_view, MainView::Changes);
    assert_eq!(layout.sidebar_width, 200.0);
    assert_eq!(layout.terminal_height, 600.0);
    assert!(layout.terminal_visible);
    let history = ProjectLayout {
      main_view: MainView::History,
      ..Default::default()
    }
    .sanitized(false);
    assert_eq!(history.main_view, MainView::History);
  }

  #[test]
  fn wire_names_keep_the_old_keys() {
    let json = serde_json::to_string(&ProjectLayout {
      panel_tab: PanelTab::GitOutput,
      ..Default::default()
    })
    .unwrap();
    assert!(json.contains("\"panelTab\":\"git-output\""));
    assert!(json.contains("\"sidebarView\":\"scm\""));
    assert!(json.contains("\"mainView\":\"changes\""));
  }

  #[test]
  fn round_trips_per_project_file() {
    let dir = tempfile::TempDir::new().unwrap();
    let layout = ProjectLayout {
      sidebar_width: 420.0,
      terminal_maximized: true,
      ..Default::default()
    };
    save_layout(dir.path(), "/repos/a", &layout).unwrap();
    assert_eq!(
      load_layout(dir.path(), "/repos/a", false),
      layout.clone().sanitized(false)
    );
    assert_ne!(layout_path(dir.path(), "/repos/a"), layout_path(dir.path(), "/repos/b"));
    assert_eq!(
      load_layout(dir.path(), "/repos/missing", false),
      ProjectLayout::default()
    );
  }
}
