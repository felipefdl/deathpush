use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::invalidate_status;
use crate::commands::repository::AppRepoState;
use crate::commands::update_window_title;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::types::RepositoryIdentity;

#[tauri::command]
pub async fn clone_repository(
  url: String,
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  registry: State<'_, RepositoryRuntimeRegistry>,
  window: WebviewWindow,
) -> Result<RepositoryIdentity> {
  let label = window.label().to_string();
  let target = PathBuf::from(&path);
  GitCli::clone_repo(&url, &target).await?;

  let repo_root = registry.open_for_window(&label, &target, &window)?;
  let repo = registry.with_runtime(&label, |runtime| runtime.open_repository())?;

  let identity = RepositoryIdentity {
    root: repo.root().to_string_lossy().to_string(),
    head_branch: repo.head_branch(),
  };
  update_window_title(&window, &identity.root, identity.head_branch.as_deref());

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(identity)
}

#[tauri::command]
pub async fn merge_continue(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.merge_continue().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn merge_abort(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.merge_abort().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn rebase_continue(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.rebase_continue().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn rebase_abort(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.rebase_abort().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn rebase_skip(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.rebase_skip().await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn merge_branch(name: String, state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.merge_branch(&name).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn rebase_branch(name: String, state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.rebase_branch(&name).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn init_repository(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  registry: State<'_, RepositoryRuntimeRegistry>,
  window: WebviewWindow,
) -> Result<RepositoryIdentity> {
  let label = window.label().to_string();
  let target = PathBuf::from(&path);
  GitCli::init_repository(&target).await?;

  let repo_root = registry.open_for_window(&label, &target, &window)?;
  let repo = registry.with_runtime(&label, |runtime| runtime.open_repository())?;

  let identity = RepositoryIdentity {
    root: repo.root().to_string_lossy().to_string(),
    head_branch: repo.head_branch(),
  };
  update_window_title(&window, &identity.root, identity.head_branch.as_deref());

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(identity)
}

#[tauri::command]
pub async fn cherry_pick(
  commit_id: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.cherry_pick(&commit_id).await?;
  invalidate_status(state.inner(), &window)
}

#[tauri::command]
pub async fn reset_to_commit(
  id: String,
  mode: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.reset_to_commit(&id, &mode).await?;
  invalidate_status(state.inner(), &window)
}
