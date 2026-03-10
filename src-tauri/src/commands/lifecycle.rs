use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::refresh_status;
use crate::commands::repository::AppRepoState;
use crate::commands::update_window_title;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::git::watcher::{self, WatcherState};
use crate::types::RepositoryStatus;

#[tauri::command]
pub async fn clone_repository(
  url: String,
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  watcher_state: State<'_, WatcherState>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let label = window.label().to_string();
  let target = PathBuf::from(&path);
  GitCli::clone_repo(&url, &target).await?;

  let repo = GitRepository::open(&target)?;
  let repo_root = repo.root().to_path_buf();
  let status = get_repository_status(&repo)?;

  // Stop old watcher for this window, start new one
  {
    let mut watchers = watcher_state.lock().map_err(|e| Error::Other(e.to_string()))?;
    watchers.remove(&label);
  }
  if let Err(err) = watcher::start_watcher(&window, &repo_root, &watcher_state) {
    tracing::warn!("failed to start watcher: {:?}", err);
  }

  update_window_title(&window, &status);

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(status)
}

#[tauri::command]
pub async fn merge_continue(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.merge_continue().await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn merge_abort(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.merge_abort().await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn rebase_continue(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.rebase_continue().await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn rebase_abort(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.rebase_abort().await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn rebase_skip(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.rebase_skip().await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn merge_branch(
  name: String,
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
  cli.merge_branch(&name).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  update_window_title(&window, &status);
  win_state.repo = Some(repo);
  Ok(status)
}

#[tauri::command]
pub async fn rebase_branch(
  name: String,
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
  cli.rebase_branch(&name).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  update_window_title(&window, &status);
  win_state.repo = Some(repo);
  Ok(status)
}

#[tauri::command]
pub async fn init_repository(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  watcher_state: State<'_, WatcherState>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let label = window.label().to_string();
  let target = PathBuf::from(&path);
  GitCli::init_repository(&target).await?;

  let repo = GitRepository::open(&target)?;
  let repo_root = repo.root().to_path_buf();
  let status = get_repository_status(&repo)?;

  {
    let mut watchers = watcher_state.lock().map_err(|e| Error::Other(e.to_string()))?;
    watchers.remove(&label);
  }
  if let Err(err) = watcher::start_watcher(&window, &repo_root, &watcher_state) {
    tracing::warn!("failed to start watcher: {:?}", err);
  }

  update_window_title(&window, &status);

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(status)
}

#[tauri::command]
pub async fn cherry_pick(
  commit_id: String,
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
  cli.cherry_pick(&commit_id).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}

#[tauri::command]
pub async fn reset_to_commit(
  id: String,
  mode: String,
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
  cli.reset_to_commit(&id, &mode).await?;
  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  refresh_status(&mut guard, &label)
}
