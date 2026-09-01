use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use tauri::{Emitter, WebviewWindow};

use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::git::watcher::{self, WatcherHandle};
use crate::types::RepositoryStatus;

pub struct RepositoryRuntime {
  root: PathBuf,
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
    let repo = self.open_repository()?;
    get_repository_status(&repo)
  }
}

#[derive(Default)]
struct RegistryState {
  runtimes: HashMap<PathBuf, Arc<RepositoryRuntime>>,
  windows: HashMap<String, PathBuf>,
  inflight: HashMap<PathBuf, Arc<OnceLock<Arc<RepositoryRuntime>>>>,
}

#[derive(Default)]
pub struct RepositoryRuntimeRegistry {
  state: Mutex<RegistryState>,
}

impl RepositoryRuntimeRegistry {
  pub fn open_for_window(&self, label: &str, path: &Path, window: &WebviewWindow) -> Result<PathBuf> {
    self.open_with(label, path, |root| match watcher::start_watcher(window, root) {
      Ok(watcher) => Some(watcher),
      Err(err) => {
        tracing::warn!("failed to start watcher: {:?}", err);
        let _ = window.emit(
          "watcher:error",
          format!("File watching unavailable: {}. Changes won't auto-refresh.", err),
        );
        None
      }
    })?;
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
    start_watcher: impl FnOnce(&Path) -> Option<WatcherHandle>,
  ) -> Result<Arc<RepositoryRuntime>> {
    let repo = GitRepository::open(path)?;
    let root = std::fs::canonicalize(repo.root())?;

    let slot = {
      let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
      if let Some(runtime) = state.runtimes.get(&root).cloned() {
        Self::bind_window(&mut state, label, &root);
        return Ok(runtime);
      }
      state
        .inflight
        .entry(root.clone())
        .or_insert_with(|| Arc::new(OnceLock::new()))
        .clone()
    };

    let runtime = slot
      .get_or_init(|| {
        Arc::new(RepositoryRuntime {
          root: root.clone(),
          _watcher: start_watcher(&root),
        })
      })
      .clone();

    let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
    state.runtimes.entry(root.clone()).or_insert_with(|| runtime.clone());
    state.inflight.remove(&root);
    Self::bind_window(&mut state, label, &root);
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
      .open_with(label, path, start_watcher)
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
    let start = Barrier::new(2);

    std::thread::scope(|scope| {
      scope.spawn(|| {
        start.wait();
        registry
          .open_for_window_with("first", directory.path(), |_| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            watcher_count.fetch_add(1, Ordering::SeqCst);
            Some(WatcherHandle::for_test())
          })
          .unwrap();
      });
      scope.spawn(|| {
        start.wait();
        registry
          .open_for_window_with("second", directory.path(), |_| {
            std::thread::sleep(std::time::Duration::from_millis(50));
            watcher_count.fetch_add(1, Ordering::SeqCst);
            Some(WatcherHandle::for_test())
          })
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
}
