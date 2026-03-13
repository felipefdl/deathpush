use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::refresh_status;
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::hunk;
use crate::types::{FileDiffWithHunks, RepositoryStatus};

#[tauri::command]
pub async fn stage_files(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.stage_files(&paths).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stage_all(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.stage_all().await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn unstage_files(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.unstage_files(&paths).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn unstage_all(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.unstage_all().await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn discard_changes(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.discard_changes(&paths).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn get_file_hunks(
  path: String,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<FileDiffWithHunks> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let hunks = hunk::parse_unified_diff(&diff_output);
  Ok(FileDiffWithHunks { path, hunks })
}

#[tauri::command]
pub async fn stage_hunk(
  path: String,
  hunk_index: usize,
  staged: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index)?;

  if staged {
    cli.apply_patch(&patch, true, true).await?;
  } else {
    cli.apply_patch(&patch, true, false).await?;
  }

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn discard_hunk(
  path: String,
  hunk_index: usize,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, false).await?;
  let patch = hunk::generate_hunk_patch(&path, &diff_output, hunk_index)?;

  // Apply the patch in reverse to the working tree (not cached)
  cli.apply_patch(&patch, false, true).await?;

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
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
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.get_unified_diff(&path, staged).await?;
  let patch = hunk::generate_lines_patch(&path, &diff_output, hunk_index, line_start, line_end)?;

  if staged {
    cli.apply_patch(&patch, true, true).await?;
  } else {
    cli.apply_patch(&patch, true, false).await?;
  }

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}
