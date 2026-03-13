use std::path::PathBuf;

use deathpush_core::error::Error;
use deathpush_core::git::cli::GitCli;
use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::git::watcher;
use deathpush_core::types::RepositoryStatus;

use crate::session::{get_event_sink, get_root, manager, refresh_status};

#[uniffi::export]
pub fn clone_repository(session_id: String, url: String, path: String) -> Result<RepositoryStatus, Error> {
  let mgr = manager();
  let target = PathBuf::from(&path);
  let event_sink = get_event_sink();
  mgr.runtime.block_on(GitCli::clone_repo(&url, &target, &event_sink))?;

  let repo = GitRepository::open(&target)?;
  let repo_root = repo.root().to_path_buf();
  let status = get_repository_status(&repo)?;

  // Stop old watcher, start new one
  {
    let mut watchers = mgr.watcher_state.lock().map_err(|e| Error::other(e.to_string()))?;
    watchers.remove(&session_id);
  }
  if let Some(sink) = &event_sink {
    if let Err(err) = watcher::start_watcher(&session_id, sink.clone(), &repo_root, &mgr.watcher_state) {
      tracing::warn!("failed to start watcher: {:?}", err);
    }
  }

  let mut sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.entry(session_id).or_default();
  state.cli_root = Some(repo_root);
  state.repo = Some(repo);

  Ok(status)
}

#[uniffi::export]
pub fn init_repository(session_id: String, path: String) -> Result<RepositoryStatus, Error> {
  let mgr = manager();
  let target = PathBuf::from(&path);
  let event_sink = get_event_sink();
  mgr.runtime.block_on(GitCli::init_repository(&target, &event_sink))?;

  let repo = GitRepository::open(&target)?;
  let repo_root = repo.root().to_path_buf();
  let status = get_repository_status(&repo)?;

  {
    let mut watchers = mgr.watcher_state.lock().map_err(|e| Error::other(e.to_string()))?;
    watchers.remove(&session_id);
  }
  if let Some(sink) = &event_sink {
    if let Err(err) = watcher::start_watcher(&session_id, sink.clone(), &repo_root, &mgr.watcher_state) {
      tracing::warn!("failed to start watcher: {:?}", err);
    }
  }

  let mut sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.entry(session_id).or_default();
  state.cli_root = Some(repo_root);
  state.repo = Some(repo);

  Ok(status)
}

#[uniffi::export]
pub fn merge_branch(session_id: String, name: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.merge_branch(&name))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn merge_continue(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.merge_continue())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn merge_abort(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.merge_abort())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn rebase_branch(session_id: String, name: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.rebase_branch(&name))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn rebase_continue(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.rebase_continue())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn rebase_abort(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.rebase_abort())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn rebase_skip(session_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.rebase_skip())?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn cherry_pick(session_id: String, commit_id: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.cherry_pick(&commit_id))?;
  refresh_status(&session_id)
}

#[uniffi::export]
pub fn reset_to_commit(session_id: String, id: String, mode: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = GitCli::new(&root);
  manager().runtime.block_on(cli.reset_to_commit(&id, &mode))?;
  refresh_status(&session_id)
}
