use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::commands::{invalidate_status, invalidate_status_paths, update_window_title};
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::session::apply::{
  FinishApply, RefreshImpact, apply_intent, finish_apply, outcome_should_bump, stamp_outcome,
};
use crate::session::registry::{SessionHandle, SessionRegistry, force_refresh_git2_extras};
use crate::session::types::{Intent, IntentOutcome, SessionSnapshot};
use crate::types::RepositoryStatus;

#[tauri::command]
pub async fn get_session_snapshot(
  sessions: State<'_, SessionRegistry>,
  registry: State<'_, RepositoryRuntimeRegistry>,
  window: WebviewWindow,
) -> Result<SessionSnapshot> {
  let label = window.label().to_string();
  let intent_lock = sessions.intent_lock(&label);
  let _intent_guard = intent_lock.lock().await;
  let runtime = registry.runtime_for_window(&label).ok_or(Error::NoRepository)?;
  let status = runtime.status()?;
  let cursor = runtime.snapshot_cursor();
  let mut handle = sessions.handle(&label)?;
  refresh_session_lists(runtime.root(), &mut handle, &status).await?;
  handle.snapshot(&status, cursor.phase, cursor.generation, cursor.revision)
}

#[tauri::command]
pub async fn session_intent(
  intent: Intent,
  sessions: State<'_, SessionRegistry>,
  registry: State<'_, RepositoryRuntimeRegistry>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<IntentOutcome> {
  let label = window.label().to_string();
  let intent_lock = sessions.intent_lock(&label);
  let _intent_guard = intent_lock.lock().await;
  match &intent {
    Intent::OpenRepository { path } => {
      open_bound_repository(path, &state, &registry, &sessions, &window)?;
    }
    Intent::CloneRepository { url, directory } => {
      let target = crate::session::policy::clone_target_path(url, directory);
      GitCli::clone_repo(url, &target).await?;
      open_bound_repository(&target.to_string_lossy(), &state, &registry, &sessions, &window)?;
    }
    _ => {}
  }

  let runtime = registry.runtime_for_window(&label).ok_or(Error::NoRepository)?;
  let status = if intent_can_use_cached_status(&intent) {
    runtime.cached_status()?
  } else {
    runtime.status()?
  };
  let root = runtime.root().to_path_buf();
  let mut handle = sessions.handle(&label)?;
  let output = apply_intent(intent.clone(), &root, &status, &mut handle).await?;
  let should_bump = outcome_should_bump(&intent, &output);
  match finish_apply(output) {
    FinishApply::Immediate(outcome) => {
      if should_bump {
        let (generation, revision) = handle.with_mut(|session| {
          let generation = session.session_generation;
          let revision = session.bump_revision();
          (generation, revision)
        })?;
        Ok(stamp_outcome(*outcome, generation, revision))
      } else {
        Ok(*outcome)
      }
    }
    FinishApply::Refresh(RefreshImpact::Snapshot) => {
      if should_bump {
        handle.with_mut(|session| session.bump_revision())?;
      }
      invalidate_status(state.inner(), &window)?;
      let _ = runtime.refresh_status();
      let status = runtime.status()?;
      let cursor = runtime.snapshot_cursor();
      refresh_session_lists(runtime.root(), &mut handle, &status).await?;
      let snapshot = handle.snapshot(&status, cursor.phase, cursor.generation, cursor.revision)?;
      Ok(IntentOutcome::Snapshot {
        snapshot: Box::new(snapshot),
      })
    }
    FinishApply::Refresh(impact) => {
      let (generation, revision) = handle.with_mut(|session| {
        let generation = session.session_generation;
        let revision = if should_bump {
          session.bump_revision()
        } else {
          session.session_revision
        };
        (generation, revision)
      })?;
      match impact {
        RefreshImpact::StatusPaths { paths } => {
          invalidate_status_paths(state.inner(), &window, &paths)?;
          if matches!(intent, Intent::AddToGitignore { .. }) {
            runtime.invalidate_file_index();
          }
        }
        RefreshImpact::StatusRepository => {
          invalidate_status(state.inner(), &window)?;
        }
        RefreshImpact::Refs => runtime.invalidate_refs(),
        RefreshImpact::Stashes => runtime.invalidate_stashes(),
        RefreshImpact::StatusAndStashes { paths } => {
          match &paths {
            Some(paths) => invalidate_status_paths(state.inner(), &window, paths)?,
            None => invalidate_status(state.inner(), &window)?,
          }
          runtime.invalidate_stashes();
        }
        RefreshImpact::Snapshot => unreachable!("snapshot refresh handled above"),
      }
      Ok(stamp_outcome(
        IntentOutcome::Ack {
          session_generation: None,
          session_revision: None,
        },
        generation,
        revision,
      ))
    }
  }
}

fn open_bound_repository(
  path: &str,
  state: &State<'_, Mutex<AppRepoState>>,
  registry: &State<'_, RepositoryRuntimeRegistry>,
  sessions: &State<'_, SessionRegistry>,
  window: &WebviewWindow,
) -> Result<()> {
  let label = window.label().to_string();
  let repo_root = registry.open_for_window(&label, &PathBuf::from(path), window)?;
  let repo = registry.with_runtime(&label, |runtime| runtime.open_repository())?;
  let root = repo.root().to_string_lossy().to_string();
  update_window_title(window, &root, repo.head_branch().as_deref());
  let mut guard = state.lock().map_err(|err| Error::Other(err.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);
  sessions.reset(&label);
  Ok(())
}

async fn refresh_session_lists(
  root: &std::path::Path,
  handle: &mut SessionHandle<'_>,
  status: &RepositoryStatus,
) -> Result<()> {
  let stashes = GitCli::new(root).stash_list().await.unwrap_or_default();
  handle.with_mut(|session| {
    force_refresh_git2_extras(session, status);
    session.stashes = stashes;
  })?;
  Ok(())
}

fn intent_can_use_cached_status(intent: &Intent) -> bool {
  matches!(
    intent,
    Intent::OpenScmDiff { .. }
      | Intent::OpenCommitDiff { .. }
      | Intent::OpenBlame { .. }
      | Intent::ClearFile
      | Intent::SetAmend { .. }
      | Intent::SetCommitMessage { .. }
      | Intent::SetFileFilter { .. }
      | Intent::LoadMoreLog
      | Intent::OpenFileHistory { .. }
      | Intent::ClearFileHistory
      | Intent::SelectCommit { .. }
      | Intent::RefreshStatus
  )
}

#[cfg(test)]
mod tests {
  use super::{intent_can_use_cached_status, refresh_session_lists};
  use crate::session::registry::SessionRegistry;
  use crate::session::types::Intent;
  use crate::types::{RepoOperationState, RepositoryStatus, StatusPhase};
  use std::sync::Arc;

  fn init_repo() -> tempfile::TempDir {
    let directory = tempfile::TempDir::new().unwrap();
    git2::Repository::init(directory.path()).unwrap();
    directory
  }

  fn status(root: &str) -> RepositoryStatus {
    RepositoryStatus {
      root: root.into(),
      head_branch: None,
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups: vec![],
      operation_state: RepoOperationState::None,
    }
  }

  #[test]
  fn refresh_status_uses_cached_status_before_changed_snapshot() {
    assert!(intent_can_use_cached_status(&Intent::RefreshStatus));
  }

  #[test]
  fn mutating_git_intents_still_need_live_status() {
    assert!(!intent_can_use_cached_status(&Intent::StageAll));
    assert!(!intent_can_use_cached_status(&Intent::Commit { confirmed: true }));
  }

  #[tokio::test]
  async fn refresh_session_lists_does_not_write_after_generation_reset() {
    let directory = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = SessionRegistry::default();
    registry
      .with_mut("w", |state| {
        state.commit_message = "old-gen".into();
      })
      .unwrap();
    let mut handle = registry.handle("w").unwrap();
    registry.reset("w");
    registry
      .with_mut("w", |state| {
        state.commit_message = "new-gen".into();
      })
      .unwrap();
    let err = refresh_session_lists(directory.path(), &mut handle, &status(&root))
      .await
      .unwrap_err();
    assert!(err.to_string().contains("generation"), "{err}");
    registry
      .with_mut("w", |state| {
        assert_eq!(state.commit_message, "new-gen");
        assert!(state.stashes.is_empty());
      })
      .unwrap();
  }

  #[tokio::test]
  async fn snapshot_with_intent_lock_waits_on_existing_lock() {
    let directory = init_repo();
    let root = directory.path().to_string_lossy().into_owned();
    let registry = Arc::new(SessionRegistry::default());
    let lock = registry.intent_lock("w");
    let guard = lock.lock().await;
    let registry_b = registry.clone();
    let root_b = directory.path().to_path_buf();
    let status = status(&root);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let task = tokio::spawn(async move {
      started_tx.send(()).ok();
      let intent_lock = registry_b.intent_lock("w");
      let _intent_guard = intent_lock.lock().await;
      let mut handle = registry_b.handle("w").unwrap();
      refresh_session_lists(&root_b, &mut handle, &status).await.unwrap();
      handle
        .snapshot(&status, StatusPhase::Settled, 0, 0)
        .unwrap()
        .session_generation
    });
    started_rx.await.unwrap();
    tokio::task::yield_now().await;
    assert!(!task.is_finished());
    drop(guard);
    assert_eq!(task.await.unwrap(), 0);
  }
}
