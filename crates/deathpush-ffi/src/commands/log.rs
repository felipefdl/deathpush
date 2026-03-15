use deathpush_core::error::Error;
use deathpush_core::git::log;
use deathpush_core::types::{CommitDetail, CommitDiffContent, CommitEntry};

use crate::session::manager;

#[uniffi::export]
pub fn get_commit_log(session_id: String, skip: u32, limit: u32) -> Result<Vec<CommitEntry>, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_log(repo, skip as usize, limit as usize)
}

#[uniffi::export]
pub fn get_commit_detail(session_id: String, commit_id: String) -> Result<CommitDetail, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_detail(repo, &commit_id)
}

#[uniffi::export]
pub fn get_commit_file_diff(session_id: String, commit_id: String, path: String) -> Result<CommitDiffContent, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_file_diff(repo, &commit_id, &path)
}
