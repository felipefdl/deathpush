use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{Emitter, Manager, WebviewWindow};

use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::StatusScope;
use crate::git::status_coordinator::StatusCoordinator;
use crate::git::watcher::{self, WatcherHandle};
use crate::types::{PathsChanged, RepositoryStatus, StatusPatch};

pub struct RepositoryRuntime {
  root: PathBuf,
  coordinator: Arc<StatusCoordinator>,
  _watcher: Option<WatcherHandle>,
}

impl RepositoryRuntime {
  pub fn root(&self) -> &Path {
    &self.root
  }

  pub fn open_repository(&self) -> Result<GitRepository> {
    GitRepository::open(self.root())
  }

  pub fn status(&self) -> Result<RepositoryStatus> {
    self.coordinator.ensure_baseline()?;
    Ok(self.coordinator.snapshot())
  }

  pub fn invalidate(&self, scope: StatusScope) {
    self.coordinator.invalidate(scope);
  }

  pub fn invalidate_paths(&self, paths: &[String]) {
    self.coordinator.invalidate_paths(paths.iter().map(String::as_str));
  }

  pub fn snapshot_cursor(&self) -> crate::types::StatusSnapshot {
    self.coordinator.snapshot_cursor()
  }
}

struct Inflight {
  slot: Arc<OnceLock<Arc<RepositoryRuntime>>>,
  waiters: usize,
}

#[derive(Default)]
struct RegistryState {
  runtimes: HashMap<PathBuf, Arc<RepositoryRuntime>>,
  windows: HashMap<String, PathBuf>,
  inflight: HashMap<PathBuf, Inflight>,
}

#[derive(Default)]
pub struct RepositoryRuntimeRegistry {
  state: Mutex<RegistryState>,
}

impl RepositoryRuntimeRegistry {
  pub fn open_for_window(&self, label: &str, path: &Path, window: &WebviewWindow) -> Result<PathBuf> {
    let handle = window.app_handle().clone();
    self.open_with(
      label,
      path,
      move |root, coordinator| {
        let patch_handle = handle.clone();
        let paths_handle = handle.clone();
        let patch_root = root.to_path_buf();
        let paths_root = root.to_path_buf();
        coordinator.bind_emitters(
          Arc::new(move |patch: StatusPatch| {
            emit_to_runtime_windows(&patch_handle, &patch_root, "repository:status-patch", &patch);
          }),
          Arc::new(move |paths: PathsChanged| {
            emit_to_runtime_windows(&paths_handle, &paths_root, "repository:paths-changed", &paths);
          }),
        );
        let sink = coordinator.spawn_worker();
        match watcher::start_watcher(root, sink, coordinator.overflow_flag()) {
          Ok(watcher) => Some(watcher),
          Err(err) => {
            tracing::warn!("failed to start watcher: {:?}", err);
            let _ = handle.emit(
              "watcher:error",
              format!("File watching unavailable: {}. Changes won't auto-refresh.", err),
            );
            None
          }
        }
      },
      || {},
    )?;
    self.root_for_window(label).ok_or(Error::NoRepository)
  }

  pub fn root_for_window(&self, label: &str) -> Option<PathBuf> {
    self.state.lock().ok()?.windows.get(label).cloned()
  }

  pub fn runtime_for_window(&self, label: &str) -> Option<Arc<RepositoryRuntime>> {
    let state = self.state.lock().ok()?;
    let root = state.windows.get(label)?;
    state.runtimes.get(root).cloned()
  }

  pub fn window_labels_for_root(&self, root: &Path) -> Vec<String> {
    let Ok(state) = self.state.lock() else {
      return Vec::new();
    };
    state
      .windows
      .iter()
      .filter(|(_, window_root)| window_root.as_path() == root)
      .map(|(label, _)| label.clone())
      .collect()
  }

  pub fn with_runtime<T>(
    &self,
    label: &str,
    callback: impl FnOnce(&RepositoryRuntime) -> Result<T>,
  ) -> Result<T> {
    let runtime = self.runtime_for_window(label).ok_or(Error::NoRepository)?;
    callback(&runtime)
  }

  pub fn remove_window(&self, label: &str) {
    let Ok(mut state) = self.state.lock() else {
      return;
    };
    let Some(root) = state.windows.remove(label) else {
      return;
    };
    if !state.windows.values().any(|window_root| window_root == &root) {
      state.runtimes.remove(&root);
    }
  }

  fn open_with(
    &self,
    label: &str,
    path: &Path,
    start_watcher: impl FnOnce(&Path, Arc<StatusCoordinator>) -> Option<WatcherHandle>,
    on_inflight: impl FnOnce(),
  ) -> Result<Arc<RepositoryRuntime>> {
    let repo = GitRepository::open(path)?;
    let root = std::fs::canonicalize(repo.root())?;

    let slot = {
      let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
      if let Some(runtime) = state.runtimes.get(&root).cloned() {
        Self::bind_window(&mut state, label, &root);
        return Ok(runtime);
      }
      let inflight = state.inflight.entry(root.clone()).or_insert_with(|| Inflight {
        slot: Arc::new(OnceLock::new()),
        waiters: 0,
      });
      inflight.waiters += 1;
      inflight.slot.clone()
    };

    on_inflight();

    let runtime = slot
      .get_or_init(|| {
        let coordinator = Arc::new(StatusCoordinator::new(root.clone()));
        Arc::new(RepositoryRuntime {
          root: root.clone(),
          coordinator: coordinator.clone(),
          _watcher: start_watcher(&root, coordinator),
        })
      })
      .clone();

    let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
    state.runtimes.entry(root.clone()).or_insert_with(|| runtime.clone());
    Self::bind_window(&mut state, label, &root);
    if let Some(current) = state.inflight.get_mut(&root)
      && Arc::ptr_eq(&current.slot, &slot)
    {
      current.waiters -= 1;
      if current.waiters == 0 {
        state.inflight.remove(&root);
      }
    }
    Ok(runtime)
  }

  fn bind_window(state: &mut RegistryState, label: &str, root: &Path) {
    let previous_root = state.windows.insert(label.to_string(), root.to_path_buf());
    if let Some(previous_root) = previous_root
      && previous_root != root
      && !state.windows.values().any(|window_root| window_root == &previous_root)
    {
      state.runtimes.remove(&previous_root);
    }
  }

  #[cfg(test)]
  fn open_for_window_with(
    &self,
    label: &str,
    path: &Path,
    start_watcher: impl FnOnce(&Path) -> Option<WatcherHandle>,
  ) -> Result<PathBuf> {
    self
      .open_with(label, path, |root, _| start_watcher(root), || {})
      .map(|runtime| runtime.root.clone())
  }

  #[cfg(test)]
  fn open_for_window_with_inflight(
    &self,
    label: &str,
    path: &Path,
    start_watcher: impl FnOnce(&Path) -> Option<WatcherHandle>,
    on_inflight: impl FnOnce(),
  ) -> Result<PathBuf> {
    self
      .open_with(label, path, |root, _| start_watcher(root), on_inflight)
      .map(|runtime| runtime.root.clone())
  }

  #[cfg(test)]
  fn runtime_count(&self) -> usize {
    match self.state.lock() {
      Ok(state) => state.runtimes.len(),
      Err(err) => err.into_inner().runtimes.len(),
    }
  }
}

fn emit_to_runtime_windows<T: Clone + serde::Serialize>(
  handle: &tauri::AppHandle,
  root: &Path,
  event: &str,
  payload: &T,
) {
  let Some(registry) = handle.try_state::<RepositoryRuntimeRegistry>() else {
    return;
  };
  for label in registry.window_labels_for_root(root) {
    if let Some(window) = handle.get_webview_window(&label) {
      let _ = window.emit(event, payload);
    }
  }
}

#[cfg(test)]
mod tests {
  use std::sync::{Arc, Barrier};
  use std::sync::atomic::{AtomicUsize, Ordering};

  use tempfile::TempDir;

  use super::RepositoryRuntimeRegistry;
  use crate::git::watcher::WatcherHandle;

  fn git_repository() -> TempDir {
    let directory = TempDir::new().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    directory
  }

  #[test]
  fn windows_for_the_same_canonical_root_share_one_runtime_and_watcher() {
    let directory = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    let watcher_count = AtomicUsize::new(0);

    registry
      .open_for_window_with("first", directory.path(), |_| {
        watcher_count.fetch_add(1, Ordering::SeqCst);
        Some(WatcherHandle::for_test())
      })
      .unwrap();
    registry
      .open_for_window_with("second", &directory.path().join("."), |_| {
        watcher_count.fetch_add(1, Ordering::SeqCst);
        Some(WatcherHandle::for_test())
      })
      .unwrap();

    assert_eq!(watcher_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.runtime_count(), 1);
    assert!(Arc::ptr_eq(
      &registry.runtime_for_window("first").unwrap(),
      &registry.runtime_for_window("second").unwrap(),
    ));

    registry.remove_window("first");
    assert_eq!(registry.runtime_count(), 1);
    assert!(registry.runtime_for_window("second").is_some());

    registry.remove_window("second");
    assert_eq!(registry.runtime_count(), 0);
  }

  #[test]
  fn window_labels_for_root_exclude_other_repositories() {
    let first_dir = git_repository();
    let second_dir = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    let first_root = registry
      .open_for_window_with("one", first_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    registry
      .open_for_window_with("two", first_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    registry
      .open_for_window_with("other", second_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();

    let mut labels = registry.window_labels_for_root(&first_root);
    labels.sort();
    assert_eq!(labels, vec!["one".to_string(), "two".to_string()]);
    assert!(!labels.iter().any(|label| label == "other"));
  }

  #[test]
  fn with_runtime_releases_registry_lock_before_callback() {
    let directory = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_window_with("main", directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();

    registry
      .with_runtime("main", |runtime| {
        assert!(registry.state.try_lock().is_ok());
        assert_eq!(registry.root_for_window("main").as_deref(), Some(runtime.root()));
        Ok(())
      })
      .unwrap();
  }

  #[test]
  fn concurrent_opens_of_uncached_root_start_one_watcher() {
    let directory = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    let watcher_count = AtomicUsize::new(0);
    let acquired = Barrier::new(2);

    std::thread::scope(|scope| {
      scope.spawn(|| {
        registry
          .open_for_window_with_inflight(
            "first",
            directory.path(),
            |_| {
              watcher_count.fetch_add(1, Ordering::SeqCst);
              Some(WatcherHandle::for_test())
            },
            || {
              acquired.wait();
            },
          )
          .unwrap();
      });
      scope.spawn(|| {
        registry
          .open_for_window_with_inflight(
            "second",
            directory.path(),
            |_| {
              watcher_count.fetch_add(1, Ordering::SeqCst);
              Some(WatcherHandle::for_test())
            },
            || {
              acquired.wait();
            },
          )
          .unwrap();
      });
    });

    assert_eq!(watcher_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.runtime_count(), 1);
    assert!(Arc::ptr_eq(
      &registry.runtime_for_window("first").unwrap(),
      &registry.runtime_for_window("second").unwrap(),
    ));
  }

  #[test]
  fn inflight_slot_stays_until_every_opener_binds() {
    let directory = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    let watcher_count = AtomicUsize::new(0);
    let acquired = Barrier::new(2);
    let first_bound = Barrier::new(2);
    let release_second = Barrier::new(2);

    std::thread::scope(|scope| {
      scope.spawn(|| {
        registry
          .open_for_window_with_inflight(
            "first",
            directory.path(),
            |_| {
              watcher_count.fetch_add(1, Ordering::SeqCst);
              Some(WatcherHandle::for_test())
            },
            || {
              acquired.wait();
            },
          )
          .unwrap();
        first_bound.wait();
      });
      scope.spawn(|| {
        registry
          .open_for_window_with_inflight(
            "second",
            directory.path(),
            |_| {
              watcher_count.fetch_add(1, Ordering::SeqCst);
              Some(WatcherHandle::for_test())
            },
            || {
              acquired.wait();
              release_second.wait();
            },
          )
          .unwrap();
      });

      first_bound.wait();
      registry.remove_window("first");
      registry
        .open_for_window_with("third", directory.path(), |_| {
          watcher_count.fetch_add(1, Ordering::SeqCst);
          Some(WatcherHandle::for_test())
        })
        .unwrap();
      release_second.wait();
    });

    assert_eq!(watcher_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.runtime_count(), 1);
    assert!(Arc::ptr_eq(
      &registry.runtime_for_window("second").unwrap(),
      &registry.runtime_for_window("third").unwrap(),
    ));
  }
}
