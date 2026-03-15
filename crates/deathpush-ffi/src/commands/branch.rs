use deathpush_core::error::Error;
use deathpush_core::git::branch as git_branch;

use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::types::{BranchEntry, RepositoryStatus};

use crate::session::{get_root, make_cli, manager};

#[uniffi::export]
pub fn list_branches(session_id: String) -> Result<Vec<BranchEntry>, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  git_branch::list_branches(repo)
}

#[uniffi::export]
pub fn checkout_branch(session_id: String, name: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.checkout_branch(&name))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn create_branch(session_id: String, name: String, start_point: Option<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager()
    .runtime
    .block_on(cli.create_branch(&name, start_point.as_deref()))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn delete_branch(session_id: String, name: String, force: bool) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.delete_branch(&name, force))?;
  Ok(())
}

#[uniffi::export]
pub fn rename_branch(session_id: String, old_name: String, new_name: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.rename_branch(&old_name, &new_name))?;

  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(&session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn delete_remote_branch(session_id: String, remote: String, name: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let cli = make_cli(&root);
  manager().runtime.block_on(cli.delete_remote_branch(&remote, &name))?;
  Ok(())
}
