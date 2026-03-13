use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::refresh_status;
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::hunk;
use crate::types::{FileDiffWithHunks, RepositoryStatus, StashEntry};

#[tauri::command]
pub async fn get_last_commit_message(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<String> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.get_last_commit_message().await
}

#[tauri::command]
pub async fn undo_last_commit(
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
  cli.undo_last_commit().await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_save(
  message: Option<String>,
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
  cli.stash_save(message.as_deref()).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_list(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<Vec<StashEntry>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.stash_list().await
}

#[tauri::command]
pub async fn stash_apply(
  index: u32,
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
  cli.stash_apply(index).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_pop(
  index: u32,
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
  cli.stash_pop(index).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_drop(
  index: u32,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<StashEntry>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.stash_drop(index).await?;
  cli.stash_list().await
}

#[tauri::command]
pub async fn stash_save_include_untracked(
  message: Option<String>,
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
  cli.stash_save_include_untracked(message.as_deref()).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_save_staged(
  message: Option<String>,
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
  cli.stash_save_staged(message.as_deref()).await?;
  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn stash_show(
  index: u32,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<FileDiffWithHunks> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  let diff_output = cli.stash_show(index).await?;
  let hunks = hunk::parse_unified_diff(&diff_output);
  Ok(FileDiffWithHunks {
    path: format!("stash@{{{}}}", index),
    hunks,
  })
}
