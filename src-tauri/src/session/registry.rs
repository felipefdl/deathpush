use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::error::{Error, Result};
use crate::git::branch::list_branches;
use crate::git::log::{get_commit_log, last_commit_info};
use crate::git::repository::GitRepository;
use crate::git::tag::list_tags;
use crate::types::{
  BranchEntry, CommitEntry, LastCommitInfo, RepositoryStatus, ResourceGroup, StashEntry, StatusPatch, StatusPhase,
  TagEntry,
};

use super::policy::derive_actions;
use super::types::{
  COMMIT_LOG_PAGE, FileSelection, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot,
  SessionStatusEvent, SessionStatusExtras,
};

#[derive(Debug, Clone, Default)]
pub struct SessionState {
  pub session_generation: u64,
  pub session_revision: u64,
  pub selection: Option<FileSelection>,
  pub selected_commit: Option<String>,
  pub commit_detail: Option<crate::types::CommitDetail>,
  pub file_history_path: Option<String>,
  pub amend_mode: bool,
  pub commit_message: String,
  pub file_filter: String,
  pub error: Option<String>,
  pub last_commit: Option<LastCommitInfo>,
  pub cached_head: Option<String>,
  pub branches: Vec<BranchEntry>,
  pub stashes: Vec<StashEntry>,
  pub tags: Vec<TagEntry>,
  pub commit_log: Vec<CommitEntry>,
  pub diff_path: Option<String>,
  pub diff_staged: bool,
}

impl SessionState {
  pub fn bump_revision(&mut self) -> u64 {
    self.session_revision = self.session_revision.saturating_add(1);
    self.session_revision
  }
}

pub struct SessionHandle<'a> {
  registry: &'a SessionRegistry,
  label: String,
  generation: u64,
}

pub trait SessionAccess {
  fn with_mut<T>(&mut self, f: impl FnOnce(&mut SessionState) -> T) -> Result<T>;
}

impl SessionAccess for SessionState {
  fn with_mut<T>(&mut self, f: impl FnOnce(&mut SessionState) -> T) -> Result<T> {
    Ok(f(self))
  }
}

impl SessionHandle<'_> {
  pub fn with_mut<T>(&mut self, f: impl FnOnce(&mut SessionState) -> T) -> Result<T> {
    self.registry.with_mut(&self.label, |state| {
      if state.session_generation != self.generation {
        Err(Error::Other("session generation mismatch".into()))
      } else {
        Ok(f(state))
      }
    })?
  }

  pub fn snapshot(
    &mut self,
    status: &RepositoryStatus,
    phase: StatusPhase,
    status_generation: u64,
    status_revision: u64,
  ) -> Result<SessionSnapshot> {
    self.with_mut(|state| {
      refresh_git2_extras(state, status);
      prune_selection(state, &status.groups);
      build_snapshot(status, phase, status_generation, status_revision, state)
    })
  }
}

impl SessionAccess for SessionHandle<'_> {
  fn with_mut<T>(&mut self, f: impl FnOnce(&mut SessionState) -> T) -> Result<T> {
    SessionHandle::with_mut(self, f)
  }
}

#[derive(Default)]
pub struct SessionRegistry {
  windows: Mutex<HashMap<String, SessionState>>,
  intent_locks: Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>,
}

impl SessionRegistry {
  pub fn reset(&self, label: &str) {
    if let Ok(mut map) = self.windows.lock() {
      let session_generation = map
        .get(label)
        .map(|state| state.session_generation.saturating_add(1))
        .unwrap_or(0);
      map.insert(
        label.to_string(),
        SessionState {
          session_generation,
          ..SessionState::default()
        },
      );
    }
  }

  pub fn remove(&self, label: &str) {
    if let Ok(mut map) = self.windows.lock() {
      map.remove(label);
    }
    let mut locks = self.intent_locks.lock().unwrap_or_else(|err| err.into_inner());
    locks.remove(label);
  }

  pub fn with_mut<T>(&self, label: &str, callback: impl FnOnce(&mut SessionState) -> T) -> Result<T> {
    let mut map = self.windows.lock().map_err(|err| Error::Other(err.to_string()))?;
    let state = map.entry(label.to_string()).or_default();
    Ok(callback(state))
  }

  pub fn intent_lock(&self, label: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut map = self.intent_locks.lock().unwrap_or_else(|err| err.into_inner());
    map
      .entry(label.to_string())
      .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
      .clone()
  }

  pub fn handle(&self, label: &str) -> Result<SessionHandle<'_>> {
    let generation = self.with_mut(label, |state| state.session_generation)?;
    Ok(SessionHandle {
      registry: self,
      label: label.to_string(),
      generation,
    })
  }

  pub fn status_event(
    &self,
    label: &str,
    status: &RepositoryStatus,
    phase: StatusPhase,
    patch: &StatusPatch,
  ) -> Result<SessionStatusEvent> {
    self.with_mut(label, |state| {
      let head_changed = patch
        .metadata
        .as_ref()
        .is_some_and(|meta| state.cached_head.as_deref() != meta.head_commit.as_deref());
      if head_changed {
        refresh_git2_extras(state, status);
      }
      prune_selection(state, &status.groups);
      build_status_event(status, phase, state, head_changed, patch.generation, patch.revision)
    })
  }
}

pub fn refresh_git2_extras(state: &mut SessionState, status: &RepositoryStatus) {
  if state.cached_head == status.head_commit && state.last_commit.is_some() {
    return;
  }
  state.cached_head = status.head_commit.clone();
  let Ok(repo) = GitRepository::open(std::path::Path::new(&status.root)) else {
    return;
  };
  state.last_commit = last_commit_info(&repo);
  state.branches = list_branches(&repo).unwrap_or_default();
  state.tags = list_tags(&repo).unwrap_or_default();
  if state.file_history_path.is_none() {
    state.commit_log = get_commit_log(&repo, 0, COMMIT_LOG_PAGE).unwrap_or_default();
  }
}

pub fn force_refresh_git2_extras(state: &mut SessionState, status: &RepositoryStatus) {
  state.cached_head = None;
  refresh_git2_extras(state, status);
}

fn prune_selection(state: &mut SessionState, groups: &[ResourceGroup]) {
  let Some(selection) = &state.selection else {
    return;
  };
  let exists = groups
    .iter()
    .any(|group| group.kind == selection.group_kind && group.files.iter().any(|file| file.path == selection.path));
  if !exists {
    state.selection = None;
    state.diff_path = None;
  }
}

pub fn build_snapshot(
  status: &RepositoryStatus,
  phase: StatusPhase,
  status_generation: u64,
  status_revision: u64,
  state: &SessionState,
) -> SessionSnapshot {
  SessionSnapshot {
    session_generation: state.session_generation,
    session_revision: state.session_revision,
    status_generation,
    status_revision,
    repo: session_repo(status, phase),
    groups: status.groups.clone(),
    selection: session_selection(state),
    scm: SessionScm {
      amend_mode: state.amend_mode,
      commit_message: state.commit_message.clone(),
      file_filter: state.file_filter.clone(),
    },
    actions: session_actions(status, state),
    last_commit: state.last_commit.clone(),
    branches: state.branches.clone(),
    stashes: state.stashes.clone(),
    tags: state.tags.clone(),
    commit_log: state.commit_log.clone(),
    commit_detail: state.commit_detail.clone(),
    file_history_path: state.file_history_path.clone(),
    error: state.error.clone(),
  }
}

pub fn build_status_event(
  status: &RepositoryStatus,
  phase: StatusPhase,
  state: &SessionState,
  include_extras: bool,
  status_generation: u64,
  status_revision: u64,
) -> SessionStatusEvent {
  SessionStatusEvent {
    session_generation: state.session_generation,
    session_revision: state.session_revision,
    status_generation,
    status_revision,
    repo: session_repo(status, phase),
    groups: status.groups.clone(),
    actions: session_actions(status, state),
    selection: session_selection(state),
    extras: include_extras.then(|| SessionStatusExtras {
      last_commit: state.last_commit.clone(),
      branches: Some(state.branches.clone()),
      tags: Some(state.tags.clone()),
      commit_log: Some(state.commit_log.clone()),
      stashes: None,
    }),
  }
}

fn session_repo(status: &RepositoryStatus, phase: StatusPhase) -> SessionRepo {
  SessionRepo {
    root: status.root.clone(),
    head_branch: status.head_branch.clone(),
    head_commit: status.head_commit.clone(),
    ahead: status.ahead,
    behind: status.behind,
    operation_state: status.operation_state,
    phase,
  }
}

fn session_selection(state: &SessionState) -> SessionSelection {
  SessionSelection {
    file: state.selection.clone(),
    commit: state.selected_commit.clone(),
  }
}

fn session_actions(status: &RepositoryStatus, state: &SessionState) -> SessionActions {
  derive_actions(
    &status.groups,
    &state.commit_message,
    state.amend_mode,
    status.ahead,
    status.behind,
    status.head_branch.is_some(),
    status.operation_state,
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{RepoOperationState, RepositoryMetadata};
  use std::path::Path;

  fn init_repo() -> (tempfile::TempDir, String) {
    let directory = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    let root = repo.workdir().unwrap();
    std::fs::write(root.join("README.md"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let oid = repo.commit(Some("HEAD"), &sig, &sig, "initial\n", &tree, &[]).unwrap();
    (directory, oid.to_string())
  }

  fn status(root: &str, head: &str) -> RepositoryStatus {
    RepositoryStatus {
      root: root.into(),
      head_branch: Some("master".into()),
      head_commit: Some(head.into()),
      ahead: 0,
      behind: 0,
      groups: vec![],
      operation_state: RepoOperationState::None,
    }
  }

  fn patch(root: &str, head: Option<&str>) -> StatusPatch {
    StatusPatch {
      generation: 1,
      base_revision: 0,
      revision: 1,
      upserts: vec![],
      removals: vec![],
      metadata: head.map(|commit| RepositoryMetadata {
        root: root.into(),
        head_branch: Some("master".into()),
        head_commit: Some(commit.into()),
        ahead: 0,
        behind: 0,
        operation_state: RepoOperationState::None,
      }),
      phase: StatusPhase::Settled,
    }
  }

  #[test]
  fn status_event_omits_extras_when_head_is_unchanged() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.cached_head = Some(oid.clone());
        state.last_commit = Some(LastCommitInfo {
          short_id: "old".into(),
          message: "stale".into(),
          author_date: "0".into(),
        });
      })
      .unwrap();
    let event = registry
      .status_event(
        "w",
        &status(&root, &oid),
        StatusPhase::Settled,
        &patch(&root, Some(&oid)),
      )
      .unwrap();
    assert!(event.extras.is_none());
    let json = serde_json::to_string(&event).unwrap();
    assert!(!json.contains("extras"), "{json}");
  }

  #[test]
  fn status_event_projects_extras_when_head_changes() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    let event = registry
      .status_event(
        "w",
        &status(&root, &oid),
        StatusPhase::Settled,
        &patch(&root, Some(&oid)),
      )
      .unwrap();
    let extras = event.extras.expect("head change must project extras");
    assert_eq!(
      extras.last_commit.as_ref().map(|commit| commit.message.trim()),
      Some("initial")
    );
    assert!(extras.branches.as_ref().unwrap().iter().any(|branch| branch.is_head));
    let commit_log = extras.commit_log.as_ref().unwrap();
    assert_eq!(commit_log.len(), 1);
    assert_eq!(commit_log[0].message.trim(), "initial");
  }

  #[test]
  fn status_event_head_change_omits_stash_key() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.stashes = vec![crate::types::StashEntry {
          index: 0,
          message: "keep me".into(),
        }];
      })
      .unwrap();
    let event = registry
      .status_event(
        "w",
        &status(&root, &oid),
        StatusPhase::Settled,
        &patch(&root, Some(&oid)),
      )
      .unwrap();
    let extras = event.extras.expect("head change must project extras");
    assert!(extras.stashes.is_none(), "HEAD extras must omit stashes: {extras:?}");
    let json = serde_json::to_string(&extras).unwrap();
    assert!(!json.contains("stashes"), "{json}");
    assert!(json.contains("branches"), "{json}");
  }

  #[test]
  fn status_event_omits_extras_when_patch_has_no_metadata() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    let event = registry
      .status_event("w", &status(&root, &oid), StatusPhase::Settled, &patch(&root, None))
      .unwrap();
    assert!(event.extras.is_none());
  }

  #[test]
  fn bump_revision_is_monotonic() {
    let mut state = SessionState::default();
    assert_eq!(state.session_revision, 0);
    assert_eq!(state.bump_revision(), 1);
    assert_eq!(state.bump_revision(), 2);
  }

  #[test]
  fn status_event_projects_session_revision() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.session_generation = 2;
        state.session_revision = 9;
      })
      .unwrap();
    let event = registry
      .status_event(
        "w",
        &status(&root, &oid),
        StatusPhase::Settled,
        &patch(&root, Some(&oid)),
      )
      .unwrap();
    assert_eq!(event.session_generation, 2);
    assert_eq!(event.session_revision, 9);
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"sessionGeneration\":2"), "{json}");
    assert!(json.contains("\"sessionRevision\":9"), "{json}");
  }

  #[test]
  fn snapshot_projects_session_revision() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.session_generation = 5;
        state.session_revision = 4;
        state.cached_head = Some(oid.clone());
        state.last_commit = Some(LastCommitInfo {
          short_id: "keep".into(),
          message: "keep".into(),
          author_date: "0".into(),
        });
      })
      .unwrap();
    let mut handle = registry.handle("w").unwrap();
    let snapshot = handle
      .snapshot(&status(&root, &oid), StatusPhase::Settled, 0, 0)
      .unwrap();
    assert_eq!(snapshot.session_generation, 5);
    assert_eq!(snapshot.session_revision, 4);
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(json.contains("\"sessionGeneration\":5"), "{json}");
    assert!(json.contains("\"sessionRevision\":4"), "{json}");
  }

  #[test]
  fn status_event_projects_status_patch_cursors() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.session_generation = 2;
        state.session_revision = 5;
      })
      .unwrap();
    let mut patch = patch(&root, Some(&oid));
    patch.generation = 4;
    patch.revision = 9;
    let event = registry
      .status_event("w", &status(&root, &oid), StatusPhase::Settled, &patch)
      .unwrap();
    assert_eq!(event.status_generation, 4);
    assert_eq!(event.status_revision, 9);
    assert_eq!(event.session_generation, 2);
    assert_eq!(event.session_revision, 5);
  }

  #[test]
  fn snapshot_projects_status_cursor() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    let mut handle = registry.handle("w").unwrap();
    let snapshot = handle
      .snapshot(&status(&root, &oid), StatusPhase::Settled, 4, 9)
      .unwrap();
    assert_eq!(snapshot.status_generation, 4);
    assert_eq!(snapshot.status_revision, 9);
  }

  #[test]
  fn refs_extras_omit_last_commit_key() {
    let extras = SessionStatusExtras {
      last_commit: None,
      branches: Some(vec![]),
      tags: Some(vec![]),
      commit_log: None,
      stashes: None,
    };
    let json = serde_json::to_string(&extras).unwrap();
    assert!(!json.contains("lastCommit"), "{json}");
    assert!(json.contains("branches"), "{json}");
    assert!(json.contains("tags"), "{json}");
  }

  #[test]
  fn reset_advances_generation_and_clears_revision() {
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.session_generation = 3;
        state.session_revision = 9;
        state.commit_message = "keep".into();
      })
      .unwrap();
    registry.reset("w");
    registry
      .with_mut("w", |state| {
        assert_eq!(state.session_generation, 4);
        assert_eq!(state.session_revision, 0);
        assert_eq!(state.commit_message, "");
      })
      .unwrap();
  }

  #[test]
  fn reset_unknown_window_starts_at_generation_zero() {
    let registry = SessionRegistry::default();
    registry.reset("fresh");
    registry
      .with_mut("fresh", |state| {
        assert_eq!(state.session_generation, 0);
        assert_eq!(state.session_revision, 0);
      })
      .unwrap();
  }

  #[tokio::test]
  async fn reset_during_handle_use_does_not_commit() {
    let registry = SessionRegistry::default();
    registry.with_mut("w", |s| s.commit_message = "old".into()).unwrap();
    let mut handle = registry.handle("w").unwrap();
    registry.reset("w");
    let err = handle
      .with_mut(|s| {
        s.commit_message = "from-old-gen".into();
      })
      .unwrap_err();
    assert!(err.to_string().contains("generation"), "{err}");
    registry
      .with_mut("w", |s| {
        assert_eq!(s.commit_message, "");
        assert_eq!(s.session_generation, 1);
      })
      .unwrap();
  }

  #[tokio::test]
  async fn overlapping_intents_on_one_window_serialize() {
    let registry = std::sync::Arc::new(SessionRegistry::default());
    registry.with_mut("w", |s| s.commit_message = "before".into()).unwrap();
    let lock = registry.intent_lock("w");
    let guard = lock.lock().await;
    let registry_b = registry.clone();
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
      let lock = registry_b.intent_lock("w");
      started_tx.send(()).ok();
      let _guard = lock.lock().await;
      registry_b.with_mut("w", |s| s.commit_message.clone()).unwrap()
    });
    started_rx.await.unwrap();
    tokio::task::yield_now().await;
    registry.with_mut("w", |s| s.commit_message = "after-a".into()).unwrap();
    drop(guard);
    let seen = task.await.unwrap();
    assert_eq!(seen, "after-a");
  }

  #[tokio::test]
  async fn two_windows_do_not_share_an_intent_lock() {
    let registry = SessionRegistry::default();
    let a = registry.intent_lock("a");
    let b = registry.intent_lock("b");
    assert!(!std::sync::Arc::ptr_eq(&a, &b));
  }

  #[tokio::test]
  async fn status_event_does_not_take_intent_lock() {
    let (directory, oid) = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    let lock = registry.intent_lock("w");
    let _guard = lock.lock().await;
    let event = registry.status_event(
      "w",
      &status(&root, &oid),
      StatusPhase::Settled,
      &patch(&root, Some(&oid)),
    );
    assert!(event.is_ok(), "{event:?}");
  }

  #[test]
  fn session_state_implements_session_access() {
    let mut state = SessionState::default();
    SessionAccess::with_mut(&mut state, |s| s.commit_message = "direct".into()).unwrap();
    assert_eq!(state.commit_message, "direct");
  }

  #[tokio::test]
  async fn remove_drops_intent_lock() {
    let registry = SessionRegistry::default();
    let first = registry.intent_lock("w");
    registry.remove("w");
    let second = registry.intent_lock("w");
    assert!(!std::sync::Arc::ptr_eq(&first, &second));
  }
}
