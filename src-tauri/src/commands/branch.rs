use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::invalidate_status;
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::branch as git_branch;
use crate::git::cli::GitCli;
use crate::types::BranchEntry;

#[tauri::command]
pub fn list_branches(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<Vec<BranchEntry>> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  git_branch::list_branches(repo)
}

#[tauri::command]
pub async fn checkout_branch(
  name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.checkout_branch(&name).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn create_branch(
  name: String,
  from: Option<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.create_branch(&name, from.as_deref()).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn delete_branch(
  name: String,
  force: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.delete_branch(&name, force).await?;
  Ok(())
}

#[tauri::command]
pub async fn rename_branch(
  old_name: String,
  new_name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.rename_branch(&old_name, &new_name).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn delete_remote_branch(
  remote: String,
  name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.delete_remote_branch(&remote, &name).await?;
  Ok(())
}
