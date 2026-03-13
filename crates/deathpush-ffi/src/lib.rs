uniffi::setup_scaffolding!();

pub mod bridge;
pub mod commands;
pub mod session;

use std::sync::Arc;

use bridge::{EventListener, FfiEventSink};
use session::{init_manager, manager};

#[uniffi::export]
pub fn initialize() {
  // Initialize tracing
  tracing_subscriber::fmt()
    .with_env_filter(
      tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
    )
    .init();

  // Resolve shell environment
  deathpush_core::shell_env::init();

  // Create tokio runtime
  let runtime = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");

  init_manager(runtime);
}

#[uniffi::export]
pub fn register_event_listener(listener: Box<dyn EventListener>) {
  let mgr = manager();
  let listener_arc: Arc<dyn EventListener> = Arc::from(listener);
  let event_sink = FfiEventSink::new(Arc::clone(&listener_arc));

  if let Ok(mut guard) = mgr.event_listener.lock() {
    *guard = Some(listener_arc);
  }
  if let Ok(mut guard) = mgr.event_sink.lock() {
    *guard = Some(event_sink);
  }
}

#[uniffi::export]
pub fn get_resolved_environment() -> std::collections::HashMap<String, String> {
  deathpush_core::shell_env::get().cloned().unwrap_or_default()
}

#[uniffi::export]
pub fn create_session(session_id: String) {
  let mgr = manager();
  if let Ok(mut sessions) = mgr.sessions.lock() {
    sessions.entry(session_id).or_default();
  }
}

#[uniffi::export]
pub fn destroy_session(session_id: String) {
  let mgr = manager();

  // Remove watcher
  if let Ok(mut watchers) = mgr.watcher_state.lock() {
    watchers.remove(&session_id);
  }

  // Remove session state
  if let Ok(mut sessions) = mgr.sessions.lock() {
    sessions.remove(&session_id);
  }
}
