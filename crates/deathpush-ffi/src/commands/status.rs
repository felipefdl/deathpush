use deathpush_core::error::Error;
use deathpush_core::git::diff;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::types::{DiffContent, RepositoryStatus};

use crate::session::manager;

#[uniffi::export]
pub fn get_status(session_id: String) -> Result<RepositoryStatus, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  get_repository_status(repo)
}

#[uniffi::export]
pub fn get_file_diff(session_id: String, path: String, staged: bool) -> Result<DiffContent, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  diff::get_file_diff(repo, &path, staged)
}
