use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::{invalidate_status, invalidate_status_paths};
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::hunk;
use crate::types::FileDiffWithHunks;

#[tauri::command]
pub async fn stage_files(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.stage_files(&paths).await?;
  invalidate_status_paths(state.inner(), &window, &paths)
}

#[tauri::command]
pub async fn stage_all(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.stage_all().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn unstage_files(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.unstage_files(&paths).await?;
  invalidate_status_paths(state.inner(), &window, &paths)
}

#[tauri::command]
pub async fn unstage_all(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.unstage_all().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn discard_changes(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.discard_changes(&paths).await?;
  invalidate_status_paths(state.inner(), &window, &paths)
}

#[tauri::command]
pub async fn get_file_hunks(
  path: String,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<FileDiffWithHunks> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let hunks = hunk::parse_unified_diff(&diff_output);
  Ok(FileDiffWithHunks { path, hunks })
}

#[tauri::command]
pub async fn get_file_patch(
  path: String,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<String> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.get_unified_diff(&path, staged).await
}

#[tauri::command]
pub async fn stage_hunk(
  path: String,
  hunk_index: usize,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index)?;

  if staged {
    cli.apply_patch(&patch, true, true).await?;
  } else {
    cli.apply_patch(&patch, true, false).await?;
  }

  invalidate_status_paths(state.inner(), &window, std::slice::from_ref(&path))
}

#[tauri::command]
pub async fn discard_hunk(
  path: String,
  hunk_index: usize,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, false).await?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index)?;

  cli.apply_patch(&patch, false, true).await?;

  invalidate_status_paths(state.inner(), &window, std::slice::from_ref(&path))
}

#[tauri::command]
pub async fn stage_lines(
  path: String,
  hunk_index: usize,
  line_start: usize,
  line_end: usize,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let patch = hunk::generate_lines_patch(&path, &diff_output, hunk_index, line_start, line_end)?;

  if staged {
    cli.apply_patch(&patch, true, true).await?;
  } else {
    cli.apply_patch(&patch, true, false).await?;
  }

  invalidate_status_paths(state.inner(), &window, std::slice::from_ref(&path))
}
