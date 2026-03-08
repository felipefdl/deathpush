use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::refresh_status;
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::types::RepositoryStatus;

#[tauri::command]
pub async fn push(
  remote: String,
  branch: String,
  force: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.push(&remote, &branch, force).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn pull(
  remote: String,
  branch: String,
  rebase: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.pull(&remote, &branch, rebase).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn fetch(
  remote: String,
  prune: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.fetch(&remote, prune).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}
