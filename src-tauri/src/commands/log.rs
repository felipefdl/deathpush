use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::log;
use crate::types::{CommitDetail, CommitDiffContent, CommitEntry};

#[tauri::command]
pub fn get_commit_log(
  skip: usize,
  limit: usize,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<CommitEntry>> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_log(repo, skip, limit)
}

#[tauri::command]
pub fn get_commit_detail(
  id: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<CommitDetail> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_detail(repo, &id)
}

#[tauri::command]
pub fn get_commit_file_diff(
  commit_id: String,
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<CommitDiffContent> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  log::get_commit_file_diff(repo, &commit_id, &path)
}
