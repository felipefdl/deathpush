use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::diff;
use crate::git::status::get_repository_status;
use crate::types::{DiffContent, RepositoryStatus};

#[tauri::command]
pub fn get_status(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  get_repository_status(repo)
}

#[tauri::command]
pub fn get_file_diff(
  path: String,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<DiffContent> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  diff::get_file_diff(repo, &path, staged)
}
