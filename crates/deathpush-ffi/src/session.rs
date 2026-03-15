use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, OnceLock};

use deathpush_core::events::EventSink;
use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::git::watcher::WatcherState;
use deathpush_core::terminal::TerminalState;
use deathpush_core::types::RepositoryStatus;

use crate::bridge::EventListener;

#[derive(Default)]
pub struct SessionState {
  pub cli_root: Option<PathBuf>,
  pub repo: Option<GitRepository>,
}

pub struct SessionManager {
  pub runtime: tokio::runtime::Runtime,
  pub sessions: Mutex<HashMap<String, SessionState>>,
  pub watcher_state: WatcherState,
  pub terminal_state: TerminalState,
  pub event_listener: Mutex<Option<Arc<dyn EventListener>>>,
  pub event_sink: Mutex<Option<Arc<dyn EventSink>>>,
}

static SESSION_MANAGER: OnceLock<SessionManager> = OnceLock::new();

pub fn init_manager(runtime: tokio::runtime::Runtime) {
  let _ = SESSION_MANAGER.set(SessionManager {
    runtime,
    sessions: Mutex::new(HashMap::new()),
    watcher_state: Mutex::new(HashMap::new()),
    terminal_state: Mutex::new(HashMap::new()),
    event_listener: Mutex::new(None),
    event_sink: Mutex::new(None),
  });
}

pub fn manager() -> &'static SessionManager {
  SESSION_MANAGER
    .get()
    .expect("SessionManager not initialized -- call initialize() first")
}

// Helper: get the cli_root for a session, cloned.
pub fn get_root(session_id: &str) -> deathpush_core::error::Result<PathBuf> {
  let sessions = manager()
    .sessions
    .lock()
    .map_err(|e| deathpush_core::error::Error::other(e.to_string()))?;
  let state = sessions
    .get(session_id)
    .ok_or(deathpush_core::error::Error::NoRepository)?;
  state.cli_root.clone().ok_or(deathpush_core::error::Error::NoRepository)
}

// Helper: refresh repository status for a session (re-opens the repo).
pub fn refresh_status(session_id: &str) -> deathpush_core::error::Result<RepositoryStatus> {
  let root = get_root(session_id)?;
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager()
    .sessions
    .lock()
    .map_err(|e| deathpush_core::error::Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

// Helper: create a GitCli with the global event sink attached.
pub fn make_cli(root: &std::path::Path) -> deathpush_core::git::cli::GitCli {
  match get_event_sink() {
    Some(sink) => deathpush_core::git::cli::GitCli::with_event_sink(root, sink),
    None => deathpush_core::git::cli::GitCli::new(root),
  }
}

// Helper: get the event sink, if registered.
pub fn get_event_sink() -> Option<Arc<dyn EventSink>> {
  manager().event_sink.lock().ok().and_then(|guard| guard.clone())
}

// Helper: get the event listener Arc, if registered.
pub fn get_event_listener() -> Option<Arc<dyn EventListener>> {
  manager().event_listener.lock().ok().and_then(|guard| guard.clone())
}
