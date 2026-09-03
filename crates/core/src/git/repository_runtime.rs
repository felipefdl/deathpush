use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, OnceLock};

use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use std::sync::Weak;

use crate::error::{Error, Result};
use crate::events::{CoreEvent, EventHub};
use crate::git::branch::list_branches;
use crate::git::invalidation::GitInvalidation;
use crate::git::log::{get_commit_log, last_commit_info};
use crate::git::repository::GitRepository;
use crate::git::status::StatusScope;
use crate::git::status_coordinator::StatusCoordinator;
use crate::git::tag::list_tags;
use crate::git::watcher::{self, WatcherHandle, WatcherMessage};
use crate::session::SessionId;
use crate::session::SessionRegistry;
use crate::session::types::{COMMIT_LOG_PAGE, SessionStatusExtras};
use crate::types::{FuzzyFileResult, PathsChanged, RepositoryStatus, StashEntry, StatusPatch, StatusPhase};

pub struct RepositoryRuntime {
  root: PathBuf,
  coordinator: Arc<StatusCoordinator>,
  wake_tx: mpsc::SyncSender<WatcherMessage>,
  _watcher: Option<WatcherHandle>,
  file_index: FileIndex,
}

struct FileIndex {
  paths: Mutex<Vec<String>>,
  dirty: Arc<AtomicBool>,
  fills: AtomicU64,
  fill_lock: Mutex<()>,
}

impl FileIndex {
  fn new() -> Self {
    Self {
      paths: Mutex::new(Vec::new()),
      dirty: Arc::new(AtomicBool::new(true)),
      fills: AtomicU64::new(0),
      fill_lock: Mutex::new(()),
    }
  }
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

  pub fn cached_status(&self) -> Result<RepositoryStatus> {
    self.coordinator.cached_status()
  }

  pub fn refresh_status(&self) -> Result<crate::types::StatusSnapshot> {
    self.coordinator.force_baseline()?;
    Ok(self.coordinator.snapshot_cursor())
  }

  pub fn invalidate(&self, scope: StatusScope) {
    self.coordinator.invalidate(scope);
    self.coordinator.try_wake(&self.wake_tx);
  }

  pub fn invalidate_paths(&self, paths: &[String]) {
    self.coordinator.invalidate_paths(paths.iter().map(String::as_str));
    self.coordinator.try_wake(&self.wake_tx);
  }

  pub fn snapshot_cursor(&self) -> crate::types::StatusSnapshot {
    self.coordinator.snapshot_cursor()
  }

  pub fn invalidate_file_index(&self) {
    self.file_index.dirty.store(true, Ordering::SeqCst);
  }

  pub fn invalidate_refs(&self) {
    self.coordinator.notify_git_invalidation(GitInvalidation::Refs);
  }

  pub fn invalidate_stashes(&self) {
    self.coordinator.notify_git_invalidation(GitInvalidation::Stash);
  }

  pub fn fuzzy_find(&self, query: &str, max_results: usize) -> Result<Vec<FuzzyFileResult>> {
    let paths = self.ensure_file_index()?;
    Ok(match_fuzzy_paths(&paths, query, max_results))
  }

  fn ensure_file_index(&self) -> Result<Vec<String>> {
    if !self.file_index.dirty.load(Ordering::SeqCst) {
      return self.cached_file_index_paths();
    }
    let _guard = self
      .file_index
      .fill_lock
      .lock()
      .map_err(|err| Error::Other(err.to_string()))?;
    if !self.file_index.dirty.load(Ordering::SeqCst) {
      return self.cached_file_index_paths();
    }
    let paths = list_quick_open_paths(&self.root)?;
    *self
      .file_index
      .paths
      .lock()
      .map_err(|err| Error::Other(err.to_string()))? = paths.clone();
    self.file_index.dirty.store(false, Ordering::SeqCst);
    self.file_index.fills.fetch_add(1, Ordering::SeqCst);
    Ok(paths)
  }

  fn cached_file_index_paths(&self) -> Result<Vec<String>> {
    self
      .file_index
      .paths
      .lock()
      .map(|paths| paths.clone())
      .map_err(|err| Error::Other(err.to_string()))
  }

  #[cfg(test)]
  pub fn file_index_fills_for_test(&self) -> u64 {
    self.file_index.fills.load(Ordering::SeqCst)
  }
}

struct Inflight {
  slot: Arc<OnceLock<Arc<RepositoryRuntime>>>,
  waiters: usize,
}

#[derive(Default)]
struct RegistryState {
  runtimes: HashMap<PathBuf, Arc<RepositoryRuntime>>,
  windows: HashMap<SessionId, PathBuf>,
  inflight: HashMap<PathBuf, Inflight>,
}

/// What runtime callbacks need to reach sessions without a UI handle.
#[derive(Clone)]
pub struct RuntimeContext {
  pub hub: Arc<EventHub>,
  pub sessions: Arc<SessionRegistry>,
  pub registry: Weak<RepositoryRuntimeRegistry>,
}

#[derive(Default)]
pub struct RepositoryRuntimeRegistry {
  state: Mutex<RegistryState>,
}

impl RepositoryRuntimeRegistry {
  pub fn open_for_session(
    self: &Arc<Self>,
    id: SessionId,
    path: &Path,
    hub: Arc<EventHub>,
    sessions: Arc<SessionRegistry>,
  ) -> Result<PathBuf> {
    let ctx = RuntimeContext {
      hub,
      sessions,
      registry: Arc::downgrade(self),
    };
    self.open_with(
      id,
      path,
      move |root, coordinator, sink| {
        let patch_ctx = ctx.clone();
        let paths_ctx = ctx.clone();
        let git_ctx = ctx.clone();
        let patch_root = root.to_path_buf();
        let paths_root = root.to_path_buf();
        let git_root = root.to_path_buf();
        let coord = coordinator.clone();
        coordinator.bind_git_invalidation(Arc::new(move |kind| {
          apply_git_list_invalidation(&git_ctx, &git_root, kind);
        }));
        coordinator.bind_emitters(
          Arc::new(move |patch: StatusPatch| {
            let status = coord.snapshot();
            let phase = coord.snapshot_cursor().phase;
            emit_session_status(&patch_ctx, &patch_root, &status, phase, &patch);
          }),
          Arc::new(move |paths: PathsChanged| {
            emit_to_root_sessions(&paths_ctx, &paths_root, CoreEvent::PathsChanged(paths));
          }),
        );

        match watcher::start_watcher(root, sink, coordinator.overflow_flag()) {
          Ok(watcher) => Some(watcher),
          Err(err) => {
            tracing::warn!("failed to start watcher: {:?}", err);
            ctx.hub.broadcast(CoreEvent::WatcherError(format!(
              "File watching unavailable: {}. Changes won't auto-refresh.",
              err
            )));
            None
          }
        }
      },
      || {},
    )?;
    self.root_for_session(id).ok_or(Error::NoRepository)
  }

  pub fn root_for_session(&self, id: SessionId) -> Option<PathBuf> {
    self.state.lock().ok()?.windows.get(&id).cloned()
  }

  pub fn runtime_for_session(&self, id: SessionId) -> Option<Arc<RepositoryRuntime>> {
    let state = self.state.lock().ok()?;
    let root = state.windows.get(&id)?;
    state.runtimes.get(root).cloned()
  }

  pub fn sessions_for_root(&self, root: &Path) -> Vec<SessionId> {
    let Ok(state) = self.state.lock() else {
      return Vec::new();
    };
    state
      .windows
      .iter()
      .filter(|(_, window_root)| window_root.as_path() == root)
      .map(|(id, _)| *id)
      .collect()
  }

  pub fn runtime_for_root(&self, root: &Path) -> Option<Arc<RepositoryRuntime>> {
    self.state.lock().ok()?.runtimes.get(root).cloned()
  }

  pub fn with_runtime<T>(&self, id: SessionId, callback: impl FnOnce(&RepositoryRuntime) -> Result<T>) -> Result<T> {
    let runtime = self.runtime_for_session(id).ok_or(Error::NoRepository)?;
    callback(&runtime)
  }

  pub fn remove_session(&self, id: SessionId) {
    let Ok(mut state) = self.state.lock() else {
      return;
    };
    let Some(root) = state.windows.remove(&id) else {
      return;
    };
    if !state.windows.values().any(|window_root| window_root == &root) {
      state.runtimes.remove(&root);
    }
  }

  fn open_with(
    &self,
    id: SessionId,
    path: &Path,
    start_watcher: impl FnOnce(&Path, Arc<StatusCoordinator>, mpsc::SyncSender<WatcherMessage>) -> Option<WatcherHandle>,
    on_inflight: impl FnOnce(),
  ) -> Result<Arc<RepositoryRuntime>> {
    let repo = GitRepository::open(path)?;
    let root = std::fs::canonicalize(repo.root())?;

    let slot = {
      let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
      if let Some(runtime) = state.runtimes.get(&root).cloned() {
        Self::bind_session(&mut state, id, &root);
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
        let file_index = FileIndex::new();
        coordinator.bind_file_index_dirty(file_index.dirty.clone());
        let sink = coordinator.spawn_worker();
        Arc::new(RepositoryRuntime {
          root: root.clone(),
          coordinator: coordinator.clone(),
          wake_tx: sink.clone(),
          _watcher: start_watcher(&root, coordinator, sink),
          file_index,
        })
      })
      .clone();

    let mut state = self.state.lock().map_err(|err| Error::Other(err.to_string()))?;
    state.runtimes.entry(root.clone()).or_insert_with(|| runtime.clone());
    Self::bind_session(&mut state, id, &root);
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

  fn bind_session(state: &mut RegistryState, id: SessionId, root: &Path) {
    let previous_root = state.windows.insert(id, root.to_path_buf());
    if let Some(previous_root) = previous_root
      && previous_root != root
      && !state.windows.values().any(|window_root| window_root == &previous_root)
    {
      state.runtimes.remove(&previous_root);
    }
  }

  #[cfg(test)]
  fn open_for_session_with(
    &self,
    id: SessionId,
    path: &Path,
    start_watcher: impl FnOnce(&Path) -> Option<WatcherHandle>,
  ) -> Result<PathBuf> {
    self
      .open_with(id, path, |root, _, _| start_watcher(root), || {})
      .map(|runtime| runtime.root.clone())
  }

  #[cfg(test)]
  fn open_for_session_with_inflight(
    &self,
    id: SessionId,
    path: &Path,
    start_watcher: impl FnOnce(&Path) -> Option<WatcherHandle>,
    on_inflight: impl FnOnce(),
  ) -> Result<PathBuf> {
    self
      .open_with(id, path, |root, _, _| start_watcher(root), on_inflight)
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

fn emit_session_status(
  ctx: &RuntimeContext,
  root: &Path,
  status: &RepositoryStatus,
  phase: StatusPhase,
  patch: &StatusPatch,
) {
  let Some(registry) = ctx.registry.upgrade() else {
    return;
  };
  for id in registry.sessions_for_root(root) {
    let Ok(event) = ctx.sessions.status_event(id, status, phase, patch) else {
      continue;
    };
    ctx.hub.send(id, CoreEvent::SessionStatus(event));
  }
}

fn emit_to_root_sessions(ctx: &RuntimeContext, root: &Path, event: CoreEvent) {
  let Some(registry) = ctx.registry.upgrade() else {
    return;
  };
  for id in registry.sessions_for_root(root) {
    ctx.hub.send(id, event.clone());
  }
}

fn apply_git_list_invalidation(ctx: &RuntimeContext, root: &Path, kind: GitInvalidation) {
  let Some(registry) = ctx.registry.upgrade() else {
    return;
  };
  let ids = registry.sessions_for_root(root);
  let extras = refresh_git_lists(root, &ctx.sessions, &ids, kind);
  emit_session_status_partial(ctx, &registry, root, &extras);
}

fn emit_session_status_partial(
  ctx: &RuntimeContext,
  registry: &RepositoryRuntimeRegistry,
  root: &Path,
  extras: &SessionStatusExtras,
) {
  let Some(runtime) = registry.runtime_for_root(root) else {
    return;
  };
  let status = runtime.coordinator.snapshot();
  let cursor = runtime.coordinator.snapshot_cursor();
  for id in registry.sessions_for_root(root) {
    let Ok(mut event) = ctx.sessions.with_mut(id, |state| {
      crate::session::registry::build_status_event(
        &status,
        cursor.phase,
        state,
        false,
        cursor.generation,
        cursor.revision,
      )
    }) else {
      continue;
    };
    event.extras = Some(extras.clone());
    ctx.hub.send(id, CoreEvent::SessionStatus(event));
  }
}

fn empty_extras() -> SessionStatusExtras {
  SessionStatusExtras {
    last_commit: None,
    branches: None,
    tags: None,
    commit_log: None,
    stashes: None,
  }
}

fn list_stashes(root: &Path) -> Vec<StashEntry> {
  let mut repo = match git2::Repository::open(root) {
    Ok(repo) => repo,
    Err(_) => return Vec::new(),
  };
  let mut entries = Vec::new();
  let _ = repo.stash_foreach(|index, message, _oid| {
    entries.push(StashEntry {
      index,
      message: message.to_string(),
    });
    true
  });
  entries
}

pub(crate) fn refresh_git_lists(
  root: &Path,
  sessions: &SessionRegistry,
  ids: &[SessionId],
  kind: GitInvalidation,
) -> SessionStatusExtras {
  let Ok(repo) = GitRepository::open(root) else {
    return empty_extras();
  };
  let extras = match kind {
    GitInvalidation::Refs => SessionStatusExtras {
      last_commit: None,
      branches: Some(list_branches(&repo).unwrap_or_default()),
      tags: Some(list_tags(&repo).unwrap_or_default()),
      commit_log: None,
      stashes: None,
    },
    GitInvalidation::Stash => SessionStatusExtras {
      last_commit: None,
      branches: None,
      tags: None,
      commit_log: None,
      stashes: Some(list_stashes(root)),
    },
    GitInvalidation::Head => SessionStatusExtras {
      last_commit: last_commit_info(&repo),
      branches: Some(list_branches(&repo).unwrap_or_default()),
      tags: Some(list_tags(&repo).unwrap_or_default()),
      commit_log: Some(get_commit_log(&repo, 0, COMMIT_LOG_PAGE).unwrap_or_default()),
      stashes: None,
    },
    GitInvalidation::Ignore | GitInvalidation::Status => empty_extras(),
  };
  for id in ids {
    let _ = sessions.with_mut(*id, |state| {
      if let Some(branches) = extras.branches.as_ref() {
        state.branches = branches.clone();
      }
      if let Some(tags) = extras.tags.as_ref() {
        state.tags = tags.clone();
      }
      if let Some(stashes) = extras.stashes.as_ref() {
        state.stashes = stashes.clone();
      }
      if extras.last_commit.is_some() {
        state.last_commit = extras.last_commit.clone();
        state.cached_head = repo.head_commit_id();
      }
      if let Some(commit_log) = extras.commit_log.as_ref() {
        state.commit_log = commit_log.clone();
      }
    });
  }
  extras
}

fn list_quick_open_paths(root: &Path) -> Result<Vec<String>> {
  let repo = git2::Repository::open(root)?;
  let mut seen = HashSet::new();
  let mut paths = Vec::new();
  let index = repo.index()?;
  for entry in index.iter() {
    let path = String::from_utf8_lossy(&entry.path).replace('\\', "/");
    if path.ends_with('/') {
      continue;
    }
    if seen.insert(path.clone()) {
      paths.push(path);
    }
  }
  let mut opts = git2::StatusOptions::new();
  opts
    .include_untracked(true)
    .recurse_untracked_dirs(true)
    .include_ignored(false)
    .include_unmodified(false);
  let statuses = repo.statuses(Some(&mut opts))?;
  for entry in statuses.iter() {
    if !entry.status().is_wt_new() {
      continue;
    }
    let Ok(path) = entry.path() else {
      continue;
    };
    let path = path.replace('\\', "/");
    if seen.insert(path.clone()) {
      paths.push(path);
    }
  }
  Ok(paths)
}

fn match_fuzzy_paths(paths: &[String], query: &str, max_results: usize) -> Vec<FuzzyFileResult> {
  if query.is_empty() {
    let mut results: Vec<FuzzyFileResult> = paths
      .iter()
      .take(max_results)
      .map(|path| FuzzyFileResult {
        path: path.clone(),
        score: 0,
        match_positions: vec![],
      })
      .collect();
    results.sort_by_key(|result| result.path.to_lowercase());
    results.truncate(max_results);
    return results;
  }

  let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
  let atom = Atom::new(
    query,
    CaseMatching::Ignore,
    Normalization::Smart,
    AtomKind::Fuzzy,
    false,
  );
  let mut scored = Vec::new();
  let mut buf = Vec::new();
  for path in paths {
    let mut indices = Vec::new();
    let haystack = Utf32Str::new(path, &mut buf);
    if let Some(score) = atom.indices(haystack, &mut matcher, &mut indices) {
      scored.push(FuzzyFileResult {
        path: path.clone(),
        score: score as u32,
        match_positions: indices.iter().map(|&index| index as usize).collect(),
      });
    }
    buf.clear();
  }
  scored.sort_by_key(|result| std::cmp::Reverse(result.score));
  scored.truncate(max_results);
  scored
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicUsize, Ordering};
  use std::sync::{Arc, Barrier};

  use tempfile::TempDir;

  use super::RepositoryRuntimeRegistry;
  use crate::git::watcher::WatcherHandle;
  use crate::session::SessionId;
  use crate::types::BranchEntry;

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
      .open_for_session_with(SessionId(1), directory.path(), |_| {
        watcher_count.fetch_add(1, Ordering::SeqCst);
        Some(WatcherHandle::for_test())
      })
      .unwrap();
    registry
      .open_for_session_with(SessionId(2), &directory.path().join("."), |_| {
        watcher_count.fetch_add(1, Ordering::SeqCst);
        Some(WatcherHandle::for_test())
      })
      .unwrap();

    assert_eq!(watcher_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.runtime_count(), 1);
    assert!(Arc::ptr_eq(
      &registry.runtime_for_session(SessionId(1)).unwrap(),
      &registry.runtime_for_session(SessionId(2)).unwrap(),
    ));

    registry.remove_session(SessionId(1));
    assert_eq!(registry.runtime_count(), 1);
    assert!(registry.runtime_for_session(SessionId(2)).is_some());

    registry.remove_session(SessionId(2));
    assert_eq!(registry.runtime_count(), 0);
  }

  #[test]
  fn sessions_for_root_exclude_other_repositories() {
    let first_dir = git_repository();
    let second_dir = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    let first_root = registry
      .open_for_session_with(SessionId(1), first_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    registry
      .open_for_session_with(SessionId(2), first_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    registry
      .open_for_session_with(SessionId(3), second_dir.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();

    let mut ids = registry.sessions_for_root(&first_root);
    ids.sort();
    assert_eq!(ids, vec![SessionId(1), SessionId(2)]);
    assert!(!ids.contains(&SessionId(3)));
  }

  #[test]
  fn with_runtime_releases_registry_lock_before_callback() {
    let directory = git_repository();
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();

    registry
      .with_runtime(SessionId(1), |runtime| {
        assert!(registry.state.try_lock().is_ok());
        assert_eq!(registry.root_for_session(SessionId(1)).as_deref(), Some(runtime.root()));
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
          .open_for_session_with_inflight(
            SessionId(1),
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
          .open_for_session_with_inflight(
            SessionId(2),
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
      &registry.runtime_for_session(SessionId(1)).unwrap(),
      &registry.runtime_for_session(SessionId(2)).unwrap(),
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
          .open_for_session_with_inflight(
            SessionId(1),
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
          .open_for_session_with_inflight(
            SessionId(2),
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
      registry.remove_session(SessionId(1));
      registry
        .open_for_session_with(SessionId(3), directory.path(), |_| {
          watcher_count.fetch_add(1, Ordering::SeqCst);
          Some(WatcherHandle::for_test())
        })
        .unwrap();
      release_second.wait();
    });

    assert_eq!(watcher_count.load(Ordering::SeqCst), 1);
    assert_eq!(registry.runtime_count(), 1);
    assert!(Arc::ptr_eq(
      &registry.runtime_for_session(SessionId(2)).unwrap(),
      &registry.runtime_for_session(SessionId(3)).unwrap(),
    ));
  }

  fn commit_readme(directory: &TempDir) {
    let repo = git2::Repository::open(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    let root = repo.workdir().unwrap();
    std::fs::write(root.join("README.md"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(std::path::Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial\n", &tree, &[]).unwrap();
  }

  #[test]
  fn fuzzy_find_uses_cached_paths_without_spawning_ls_files_twice() {
    let directory = git_repository();
    commit_readme(&directory);
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    let runtime = registry.runtime_for_session(SessionId(1)).unwrap();
    let first = runtime.fuzzy_find("", 100).unwrap();
    assert!(first.iter().any(|result| result.path == "README.md"));
    let fills = runtime.file_index_fills_for_test();
    let second = runtime.fuzzy_find("README", 100).unwrap();
    assert!(second.iter().any(|result| result.path == "README.md"));
    assert_eq!(runtime.file_index_fills_for_test(), fills);
    assert_eq!(fills, 1);
  }

  #[test]
  fn gitignore_change_invalidates_index() {
    let directory = git_repository();
    commit_readme(&directory);
    std::fs::write(directory.path().join("noise.log"), "n\n").unwrap();
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    let runtime = registry.runtime_for_session(SessionId(1)).unwrap();
    let before = runtime.fuzzy_find("", 100).unwrap();
    assert!(before.iter().any(|result| result.path == "noise.log"));
    std::fs::write(directory.path().join(".gitignore"), "noise.log\n").unwrap();
    runtime.invalidate_file_index();
    let after = runtime.fuzzy_find("", 100).unwrap();
    assert!(!after.iter().any(|result| result.path == "noise.log"));
    assert!(after.iter().any(|result| result.path == "README.md"));
  }

  #[test]
  fn content_edit_does_not_invalidate_index() {
    let directory = git_repository();
    commit_readme(&directory);
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    let runtime = registry.runtime_for_session(SessionId(1)).unwrap();
    runtime.fuzzy_find("", 100).unwrap();
    let fills = runtime.file_index_fills_for_test();
    std::fs::write(directory.path().join("README.md"), "changed\n").unwrap();
    runtime.fuzzy_find("", 100).unwrap();
    assert_eq!(runtime.file_index_fills_for_test(), fills);
  }

  #[test]
  fn content_patch_does_not_refresh_branches() {
    let directory = git_repository();
    commit_readme(&directory);
    let sessions = crate::session::registry::SessionRegistry::default();
    sessions
      .with_mut(SessionId(1), |state| {
        state.branches = vec![BranchEntry {
          name: "cached".into(),
          is_head: true,
          is_remote: false,
          upstream: None,
          ahead: 0,
          behind: 0,
        }];
      })
      .unwrap();
    let extras = super::refresh_git_lists(
      directory.path(),
      &sessions,
      &[SessionId(1)],
      crate::git::invalidation::GitInvalidation::Status,
    );
    assert!(extras.branches.is_none());
    sessions
      .with_mut(SessionId(1), |state| {
        assert_eq!(state.branches[0].name, "cached");
      })
      .unwrap();
  }

  #[test]
  fn packed_refs_refresh_branches_without_head_move() {
    let directory = git_repository();
    commit_readme(&directory);
    create_branch(&directory, "feature");
    let sessions = crate::session::registry::SessionRegistry::default();
    let extras = super::refresh_git_lists(
      directory.path(),
      &sessions,
      &[SessionId(1)],
      crate::git::invalidation::GitInvalidation::Refs,
    );
    assert!(extras.branches.is_some());
    assert!(extras.tags.is_some());
    assert!(extras.last_commit.is_none());
    assert!(extras.stashes.is_none());
    assert!(extras.commit_log.is_none());
    let json = serde_json::to_string(&extras).unwrap();
    assert!(!json.contains("lastCommit"), "{json}");
    assert!(json.contains("branches"), "{json}");
    sessions
      .with_mut(SessionId(1), |state| {
        assert!(
          state.branches.iter().any(|branch| branch.name == "feature"),
          "{:?}",
          state.branches
        );
        assert!(state.last_commit.is_none());
      })
      .unwrap();
  }

  #[test]
  fn stash_ref_refresh_does_not_force_baseline() {
    let directory = git_repository();
    commit_readme(&directory);
    stash_wip(&directory, "wip");
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    let runtime = registry.runtime_for_session(SessionId(1)).unwrap();
    let groups_json = serde_json::to_string(&runtime.status().unwrap().groups).unwrap();
    let sessions = crate::session::registry::SessionRegistry::default();
    let extras = super::refresh_git_lists(
      directory.path(),
      &sessions,
      &[SessionId(1)],
      crate::git::invalidation::GitInvalidation::Stash,
    );
    assert!(extras.stashes.is_some());
    assert!(extras.branches.is_none());
    assert!(extras.last_commit.is_none());
    assert_eq!(
      serde_json::to_string(&runtime.cached_status().unwrap().groups).unwrap(),
      groups_json
    );
    sessions
      .with_mut(SessionId(1), |state| {
        assert!(!state.stashes.is_empty());
        assert!(state.stashes.iter().any(|stash| stash.message.contains("wip")));
      })
      .unwrap();
  }

  #[test]
  fn invalidate_stashes_fills_both_window_labels() {
    let directory = git_repository();
    commit_readme(&directory);
    stash_wip(&directory, "shared");
    let registry = RepositoryRuntimeRegistry::default();
    registry
      .open_for_session_with(SessionId(1), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    registry
      .open_for_session_with(SessionId(2), directory.path(), |_| Some(WatcherHandle::for_test()))
      .unwrap();
    let sessions = crate::session::registry::SessionRegistry::default();
    let extras = super::refresh_git_lists(
      directory.path(),
      &sessions,
      &[SessionId(1), SessionId(2)],
      crate::git::invalidation::GitInvalidation::Stash,
    );
    assert!(extras.stashes.is_some());
    assert!(extras.branches.is_none());
    for id in [SessionId(1), SessionId(2)] {
      sessions
        .with_mut(id, |state| {
          assert!(
            !state.stashes.is_empty(),
            "{id:?} missing stash extras: {:?}",
            state.stashes
          );
        })
        .unwrap();
    }
  }

  fn create_branch(directory: &TempDir, name: &str) {
    let repo = git2::Repository::open(directory.path()).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(name, &commit, false).unwrap();
  }

  fn stash_wip(directory: &TempDir, message: &str) {
    let mut repo = git2::Repository::open(directory.path()).unwrap();
    std::fs::write(directory.path().join("README.md"), "dirty\n").unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.stash_save(&sig, message, None).unwrap();
  }
}
