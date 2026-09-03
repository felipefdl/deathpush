use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SavedWindow {
  pub x: f32,
  pub y: f32,
  pub width: f32,
  pub height: f32,
  pub maximized: bool,
}

impl Default for SavedWindow {
  fn default() -> Self {
    Self {
      x: 0.0,
      y: 0.0,
      width: 1400.0,
      height: 900.0,
      maximized: false,
    }
  }
}

/// The last known bounds of every open window, in creation order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct WindowsState {
  pub windows: Vec<SavedWindow>,
}

impl WindowsState {
  pub fn bounds_for(&self, index: usize) -> SavedWindow {
    self.windows.get(index).copied().unwrap_or_else(|| {
      let mut window = self.windows.first().copied().unwrap_or_default();
      window.x += 30.0 * index as f32;
      window.y += 30.0 * index as f32;
      window.maximized = false;
      window
    })
  }

  pub fn record(&mut self, index: usize, window: SavedWindow) {
    if self.windows.len() <= index {
      self.windows.resize(index + 1, SavedWindow::default());
    }
    self.windows[index] = window;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn second_window_offsets_from_the_first() {
    let mut state = WindowsState::default();
    state.record(
      0,
      SavedWindow {
        x: 100.0,
        y: 50.0,
        width: 1200.0,
        height: 800.0,
        maximized: true,
      },
    );
    let second = state.bounds_for(1);
    assert_eq!((second.x, second.y), (130.0, 80.0));
    assert!(!second.maximized);
    assert_eq!(second.width, 1200.0);
  }

  #[test]
  fn record_grows_the_list() {
    let mut state = WindowsState::default();
    state.record(2, SavedWindow::default());
    assert_eq!(state.windows.len(), 3);
  }
}
