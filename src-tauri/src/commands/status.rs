use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::diff;
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::types::{DiffContent, RepositoryStatus, StatusSnapshot};

#[tauri::command]
pub fn get_status(registry: State<'_, RepositoryRuntimeRegistry>, window: WebviewWindow) -> Result<RepositoryStatus> {
  registry.with_runtime(window.label(), |runtime| runtime.status())
}

#[tauri::command]
pub fn get_status_snapshot(
  registry: State<'_, RepositoryRuntimeRegistry>,
  window: WebviewWindow,
) -> Result<StatusSnapshot> {
  registry.with_runtime(window.label(), |runtime| Ok(runtime.snapshot_cursor()))
}

#[tauri::command]
pub fn refresh_status(registry: State<'_, RepositoryRuntimeRegistry>, window: WebviewWindow) -> Result<StatusSnapshot> {
  registry.with_runtime(window.label(), |runtime| runtime.refresh_status())
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
