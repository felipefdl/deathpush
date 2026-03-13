use std::sync::Arc;

use deathpush_core::events::{EventSink, TerminalSink};

#[uniffi::export(callback_interface)]
pub trait EventListener: Send + Sync {
  fn on_repository_changed(&self, session_id: String);
  fn on_git_command(&self, command: String, duration_ms: u64, timestamp: String);
  fn on_watcher_error(&self, session_id: String, message: String);
  fn on_terminal_data(&self, session_id: u64, data: String);
  fn on_terminal_exit(&self, session_id: u64);
}

pub struct FfiEventSink {
  listener: Arc<dyn EventListener>,
}

impl FfiEventSink {
  pub fn new(listener: Arc<dyn EventListener>) -> Arc<Self> {
    Arc::new(Self { listener })
  }
}

impl EventSink for FfiEventSink {
  fn emit_git_command(&self, command: &str, duration_ms: u64, timestamp: &str) {
    self.listener.on_git_command(command.to_string(), duration_ms, timestamp.to_string());
  }

  fn emit_repository_changed(&self, session_id: &str) {
    self.listener.on_repository_changed(session_id.to_string());
  }

  fn emit_watcher_error(&self, session_id: &str, message: &str) {
    self.listener.on_watcher_error(session_id.to_string(), message.to_string());
  }
}

pub struct FfiTerminalSink {
  listener: Arc<dyn EventListener>,
}

impl FfiTerminalSink {
  pub fn new(listener: Arc<dyn EventListener>) -> Arc<Self> {
    Arc::new(Self { listener })
  }
}

impl TerminalSink for FfiTerminalSink {
  fn on_data(&self, session_id: u64, data: String) {
    self.listener.on_terminal_data(session_id, data);
  }

  fn on_exit(&self, session_id: u64) {
    self.listener.on_terminal_exit(session_id);
  }
}
