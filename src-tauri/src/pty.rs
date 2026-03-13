pub use deathpush_core::terminal::{PtySession, TerminalState};

use std::sync::Arc;

use deathpush_core::events::TerminalSink;
use tauri::{Emitter, Manager, WebviewWindow};

#[derive(serde::Serialize, Clone)]
struct TerminalDataEvent {
  id: u64,
  data: String,
}

pub struct TauriTerminalSink {
  app_handle: tauri::AppHandle,
  window_label: String,
}

impl TauriTerminalSink {
  pub fn new(window: &WebviewWindow) -> Arc<Self> {
    Arc::new(Self {
      app_handle: window.app_handle().clone(),
      window_label: window.label().to_string(),
    })
  }
}

impl TerminalSink for TauriTerminalSink {
  fn on_data(&self, session_id: u64, data: String) {
    if let Some(w) = self.app_handle.get_webview_window(&self.window_label) {
      let _ = w.emit("terminal:data", TerminalDataEvent { id: session_id, data });
    }
  }

  fn on_exit(&self, session_id: u64) {
    if let Some(w) = self.app_handle.get_webview_window(&self.window_label) {
      let _ = w.emit("terminal:exit", session_id);
    }
  }
}
