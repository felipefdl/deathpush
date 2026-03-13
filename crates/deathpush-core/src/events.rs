pub trait EventSink: Send + Sync + 'static {
  fn emit_git_command(&self, command: &str, duration_ms: u64, timestamp: &str);
  fn emit_repository_changed(&self, session_id: &str);
  fn emit_watcher_error(&self, session_id: &str, message: &str);
}

pub trait TerminalSink: Send + Sync + 'static {
  fn on_data(&self, session_id: u64, data: String);
  fn on_exit(&self, session_id: u64);
}
