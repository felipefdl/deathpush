use std::path::PathBuf;
use std::time::Duration;

use deathpush_core::config::layout::{MainView, PanelTab, ProjectLayout, SidebarView, load_layout, save_layout};
use gpui_kit::*;

use crate::config::AppConfig;

const SAVE_DEBOUNCE: Duration = Duration::from_millis(500);

/// The per-project shell layout, saved half a second after the last change.
pub struct LayoutModel {
  root: String,
  dir: PathBuf,
  layout: ProjectLayout,
  revision: u64,
}

impl LayoutModel {
  pub fn load(root: &str, cx: &App) -> Self {
    let config = AppConfig::get(cx);
    Self::load_from(
      config.dir().to_path_buf(),
      root,
      config.settings.ui.always_open_terminal_on_start,
    )
  }

  pub fn load_from(dir: PathBuf, root: &str, always_open_terminal: bool) -> Self {
    Self {
      root: root.to_string(),
      layout: load_layout(&dir, root, always_open_terminal),
      dir,
      revision: 0,
    }
  }

  pub fn layout(&self) -> &ProjectLayout {
    &self.layout
  }

  fn changed(&mut self, cx: &mut Context<Self>) {
    self.revision += 1;
    let revision = self.revision;
    cx.notify();
    cx.spawn(async move |this, cx| {
      cx.background_executor().timer(SAVE_DEBOUNCE).await;
      let _ = this.update(cx, |this, _| {
        if this.revision == revision {
          this.save_now();
        }
      });
    })
    .detach();
  }

  pub fn save_now(&self) {
    if let Err(err) = save_layout(&self.dir, &self.root, &self.layout) {
      tracing::warn!("could not save layout for {}: {err}", self.root);
    }
  }

  pub fn set_sidebar_width(&mut self, width: f32, cx: &mut Context<Self>) {
    let width = width.clamp(200.0, 600.0);
    if (self.layout.sidebar_width - width).abs() > 0.5 {
      self.layout.sidebar_width = width;
      self.changed(cx);
    }
  }

  pub fn set_terminal_height(&mut self, height: f32, cx: &mut Context<Self>) {
    let height = height.clamp(100.0, 600.0);
    if (self.layout.terminal_height - height).abs() > 0.5 {
      self.layout.terminal_height = height;
      self.changed(cx);
    }
  }

  #[allow(dead_code)]
  pub fn set_history_list_width(&mut self, width: f32, cx: &mut Context<Self>) {
    let width = width.clamp(200.0, 600.0);
    if (self.layout.history_list_width - width).abs() > 0.5 {
      self.layout.history_list_width = width;
      self.changed(cx);
    }
  }

  pub fn set_terminal_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
    if self.layout.terminal_visible == visible {
      return;
    }
    self.layout.terminal_visible = visible;
    self.changed(cx);
  }

  pub fn select_main_view(&mut self, view: MainView, cx: &mut Context<Self>) {
    self.layout.select_main_view(view);
    self.changed(cx);
  }

  pub fn select_sidebar_view(&mut self, view: SidebarView, cx: &mut Context<Self>) {
    self.layout.select_sidebar_view(view);
    self.changed(cx);
  }

  pub fn set_panel_tab(&mut self, tab: PanelTab, cx: &mut Context<Self>) {
    if self.layout.panel_tab == tab {
      return;
    }
    self.layout.panel_tab = tab;
    self.changed(cx);
  }

  pub fn toggle_terminal_maximized(&mut self, cx: &mut Context<Self>) {
    self.layout.terminal_maximized = !self.layout.terminal_maximized;
    self.changed(cx);
  }

  #[allow(dead_code)]
  pub fn dock_terminal(&mut self, cx: &mut Context<Self>) {
    if self.layout.terminal_maximized {
      self.layout.terminal_maximized = false;
      self.changed(cx);
    }
  }

  #[allow(dead_code)]
  pub fn toggle_pane_collapsed(&mut self, id: &str, cx: &mut Context<Self>) {
    self.layout.toggle_pane(id);
    self.changed(cx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  #[gpui_kit::test]
  fn changes_save_once_after_the_debounce(cx: &mut TestAppContext) {
    let dir = tempfile::TempDir::new().unwrap();
    let model = cx.new(|_| LayoutModel::load_from(dir.path().to_path_buf(), "/repos/a", false));
    model.update(cx, |model, cx| {
      model.set_sidebar_width(420.0, cx);
      model.select_main_view(MainView::History, cx);
    });
    assert_eq!(load_layout(dir.path(), "/repos/a", false).sidebar_width, 300.0);
    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    let saved = load_layout(dir.path(), "/repos/a", false);
    assert_eq!(saved.sidebar_width, 420.0);
    assert_eq!(saved.main_view, MainView::History);
  }

  #[gpui_kit::test]
  fn unchanged_terminal_visibility_and_panel_tab_do_not_save(cx: &mut TestAppContext) {
    let dir = tempfile::TempDir::new().unwrap();
    let model = cx.new(|_| LayoutModel::load_from(dir.path().to_path_buf(), "/repos/a", false));
    model.update(cx, |model, cx| {
      model.set_terminal_visible(true, cx);
      model.set_panel_tab(PanelTab::Terminal, cx);
    });
    cx.executor().advance_clock(Duration::from_millis(600));
    cx.run_until_parked();
    assert!(!deathpush_core::config::layout::layout_path(dir.path(), "/repos/a").exists());
  }
}
