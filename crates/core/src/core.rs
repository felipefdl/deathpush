use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::mpsc::UnboundedReceiver;
use tokio::task::JoinHandle;

use crate::error::Result;
use crate::events::{CoreEvent, EventHub};
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::ops::repository::RepoState;
use crate::pty::TerminalState;
use crate::session::{SessionId, SessionRegistry};
#[cfg(not(test))]
use crate::shell_env::ShellEnvResolver;

/// The one object the app talks to. Owns the tokio runtime and every registry.
pub struct Core {
  runtime: tokio::runtime::Runtime,
  pub(crate) hub: Arc<EventHub>,
  pub(crate) sessions: Arc<SessionRegistry>,
  pub(crate) runtimes: Arc<RepositoryRuntimeRegistry>,
  pub(crate) terminals: TerminalState,
  pub(crate) windows: Mutex<HashMap<SessionId, RepoState>>,
  pub(crate) resource_dir: PathBuf,
  next_session: AtomicU64,
}

impl Core {
  /// `resource_dir` holds `bin/dp` and `bin/dp.cmd` for the CLI installer.
  pub fn new(resource_dir: PathBuf) -> Result<Arc<Self>> {
    start_shell_env_once();
    let runtime = tokio::runtime::Builder::new_multi_thread()
      .enable_all()
      .thread_name("deathpush-core")
      .build()?;
    let hub = Arc::new(EventHub::default());
    let sink_hub = hub.clone();
    crate::git::cli::set_command_sink(Arc::new(move |event| {
      sink_hub.broadcast(CoreEvent::GitCommand(event));
    }));
    Ok(Arc::new(Self {
      runtime,
      hub,
      sessions: Arc::new(SessionRegistry::default()),
      runtimes: Arc::new(RepositoryRuntimeRegistry::default()),
      terminals: TerminalState::new(HashMap::new()),
      windows: Mutex::new(HashMap::new()),
      resource_dir,
      next_session: AtomicU64::new(1),
    }))
  }

  pub fn open_session(&self) -> (SessionId, UnboundedReceiver<CoreEvent>) {
    let id = SessionId(self.next_session.fetch_add(1, Ordering::Relaxed));
    (id, self.hub.subscribe(id))
  }

  pub fn close_session(&self, id: SessionId) {
    self.lock_windows().remove(&id);
    self.runtimes.remove_session(id);
    self.sessions.remove(id);
    if let Ok(mut terminals) = self.terminals.lock() {
      terminals.retain(|_, session| session.session != id);
    }
    self.hub.unsubscribe(id);
  }

  /// Runs a future on core's tokio runtime. The handle is a plain future the app can await anywhere.
  pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
  {
    self.runtime.spawn(future)
  }

  pub fn runtime_handle(&self) -> tokio::runtime::Handle {
    self.runtime.handle().clone()
  }

  pub(crate) fn lock_windows(&self) -> std::sync::MutexGuard<'_, HashMap<SessionId, RepoState>> {
    self.windows.lock().unwrap_or_else(|err| err.into_inner())
  }
}

fn start_shell_env_once() {
  // Unit tests share this process with `shell_env` tests that assert `RESOLVED_ENV` is empty.
  #[cfg(not(test))]
  {
    static START: std::sync::Once = std::sync::Once::new();
    START.call_once(|| {
      ShellEnvResolver::start();
    });
  }
}

#[cfg(test)]
mod tests {
  use super::Core;
  use crate::session::types::{Intent, IntentOutcome};

  fn init_repo() -> tempfile::TempDir {
    let directory = tempfile::TempDir::new().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    directory
  }

  #[test]
  fn open_repository_returns_a_snapshot_and_binds_the_root() {
    let directory = init_repo();
    let core = Core::new(directory.path().to_path_buf()).unwrap();
    let (id, _events) = core.open_session();
    let path = directory.path().to_string_lossy().into_owned();
    let outcome = core
      .runtime_handle()
      .block_on(core.session_intent(id, Intent::OpenRepository { path }))
      .unwrap();
    assert!(matches!(outcome, IntentOutcome::Snapshot { .. }));
    assert_eq!(
      core.repo_root(id).unwrap(),
      std::fs::canonicalize(directory.path()).unwrap()
    );
    core.close_session(id);
    assert!(core.repo_root(id).is_err());
  }

  #[test]
  fn session_ids_are_unique_and_subscribed() {
    let directory = init_repo();
    let core = Core::new(directory.path().to_path_buf()).unwrap();
    let (a, _ra) = core.open_session();
    let (b, _rb) = core.open_session();
    assert_ne!(a, b);
  }
}
