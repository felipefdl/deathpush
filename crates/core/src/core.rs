use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

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
  runtime: Mutex<Option<tokio::runtime::Runtime>>,
  pub(crate) hub: Arc<EventHub>,
  pub(crate) sessions: Arc<SessionRegistry>,
  pub(crate) runtimes: Arc<RepositoryRuntimeRegistry>,
  pub(crate) terminals: TerminalState,
  pub(crate) repos: Mutex<HashMap<SessionId, RepoState>>,
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
      runtime: Mutex::new(Some(runtime)),
      hub,
      sessions: Arc::new(SessionRegistry::default()),
      runtimes: Arc::new(RepositoryRuntimeRegistry::default()),
      terminals: TerminalState::new(HashMap::new()),
      repos: Mutex::new(HashMap::new()),
      resource_dir,
      next_session: AtomicU64::new(1),
    }))
  }

  /// Takes the tokio runtime and waits up to 2 seconds for workers to finish.
  pub fn shutdown(&self) {
    if let Some(runtime) = self.take_runtime() {
      runtime.shutdown_timeout(Duration::from_secs(2));
    }
  }

  fn take_runtime(&self) -> Option<tokio::runtime::Runtime> {
    self.runtime.lock().unwrap_or_else(|err| err.into_inner()).take()
  }

  pub fn open_session(&self) -> (SessionId, UnboundedReceiver<CoreEvent>) {
    let id = SessionId(self.next_session.fetch_add(1, Ordering::Relaxed));
    (id, self.hub.subscribe(id))
  }

  pub async fn close_session(&self, id: SessionId) {
    let intent_lock = self.sessions.intent_lock(id);
    let guard = intent_lock.lock().await;
    self.lock_repos().remove(&id);
    self.runtimes.remove_session(id);
    let doomed = {
      let mut terminals = self.terminals.lock().unwrap_or_else(|err| err.into_inner());
      let ids: Vec<u64> = terminals
        .iter()
        .filter(|(_, session)| session.session == id)
        .map(|(terminal, _)| *terminal)
        .collect();
      ids
        .into_iter()
        .filter_map(|terminal| terminals.remove(&terminal))
        .collect::<Vec<_>>()
    };
    let handle = self.runtime_handle();
    for session in doomed {
      handle.spawn_blocking(move || {
        let mut session = session;
        session.shutdown();
      });
    }
    self.hub.unsubscribe(id);
    self.sessions.remove(id);
    drop(guard);
    self.sessions.remove_intent_lock(id);
  }

  /// Runs a future on core's tokio runtime. The handle is a plain future the app can await anywhere.
  pub fn spawn<F>(&self, future: F) -> JoinHandle<F::Output>
  where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
  {
    self.runtime_handle().spawn(future)
  }

  pub fn runtime_handle(&self) -> tokio::runtime::Handle {
    self
      .runtime
      .lock()
      .unwrap_or_else(|err| err.into_inner())
      .as_ref()
      .expect("core runtime has been shut down")
      .handle()
      .clone()
  }

  pub(crate) fn lock_repos(&self) -> std::sync::MutexGuard<'_, HashMap<SessionId, RepoState>> {
    self.repos.lock().unwrap_or_else(|err| err.into_inner())
  }
}

impl Drop for Core {
  fn drop(&mut self) {
    if let Some(runtime) = self.runtime.get_mut().unwrap_or_else(|err| err.into_inner()).take() {
      runtime.shutdown_background();
    }
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
    core.runtime_handle().block_on(core.close_session(id));
    assert!(core.repo_root(id).is_err());
  }

  #[test]
  fn close_session_waits_for_in_flight_intent() {
    let directory = init_repo();
    let core = Core::new(directory.path().to_path_buf()).unwrap();
    let (id, _events) = core.open_session();
    let path = directory.path().to_string_lossy().into_owned();
    core
      .runtime_handle()
      .block_on(core.session_intent(id, Intent::OpenRepository { path }))
      .unwrap();
    let mut handle = core.sessions.handle(id).unwrap();

    let lock = core.sessions.intent_lock(id);
    let runtime = core.runtime_handle();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let lock_task = runtime.spawn(async move {
      let _guard = lock.lock().await;
      started_tx.send(()).ok();
      let _ = release_rx.await;
    });
    runtime.block_on(started_rx).unwrap();

    let close_core = core.clone();
    let close_task = runtime.spawn(async move {
      close_core.close_session(id).await;
    });
    runtime.block_on(tokio::task::yield_now());
    assert!(!close_task.is_finished(), "close_session must wait on the intent lock");

    handle
      .with_mut(|session| {
        session.commit_message = "in-flight".into();
      })
      .unwrap();

    release_tx.send(()).ok();
    runtime.block_on(async {
      close_task.await.unwrap();
      lock_task.await.unwrap();
    });

    assert!(core.repo_root(id).is_err());
    assert!(!core.sessions.contains(id));
  }

  #[test]
  fn close_session_keeps_intent_lock_arc_until_guard_drops() {
    let directory = init_repo();
    let core = Core::new(directory.path().to_path_buf()).unwrap();
    let (id, _events) = core.open_session();
    let path = directory.path().to_string_lossy().into_owned();
    core
      .runtime_handle()
      .block_on(core.session_intent(id, Intent::OpenRepository { path }))
      .unwrap();

    let first = core.sessions.intent_lock(id);
    let runtime = core.runtime_handle();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = tokio::sync::oneshot::channel();
    let lock = first.clone();
    let lock_task = runtime.spawn(async move {
      let _guard = lock.lock().await;
      started_tx.send(()).ok();
      let _ = release_rx.await;
    });
    runtime.block_on(started_rx).unwrap();

    let close_core = core.clone();
    let close_task = runtime.spawn(async move {
      close_core.close_session(id).await;
    });
    runtime.block_on(tokio::task::yield_now());
    assert!(!close_task.is_finished(), "close_session must wait on the intent lock");

    core.sessions.remove(id);
    let during = core.sessions.intent_lock(id);
    assert!(
      std::sync::Arc::ptr_eq(&first, &during),
      "intent_lock during close_session must be the same Arc"
    );

    release_tx.send(()).ok();
    runtime.block_on(async {
      close_task.await.unwrap();
      lock_task.await.unwrap();
    });
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
