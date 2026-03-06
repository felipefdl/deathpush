use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::blame;
use crate::types::{CommitEntry, FileBlame, LastCommitInfo};

#[tauri::command]
pub async fn get_file_blame(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<FileBlame> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  blame::get_file_blame(&root, &path).await
}

#[tauri::command]
pub async fn get_file_log(
  path: String,
  skip: usize,
  limit: usize,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<CommitEntry>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  blame::get_file_log(&root, &path, skip, limit).await
}

#[tauri::command]
pub async fn get_last_commit_info(
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<LastCommitInfo> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  blame::get_last_commit_info(&root).await
}
