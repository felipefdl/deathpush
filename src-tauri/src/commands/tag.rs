use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::repository::GitRepository;
use crate::git::tag as git_tag;
use crate::types::TagEntry;

#[tauri::command]
pub fn list_tags(state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<Vec<TagEntry>> {
  let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  let repo = win_state.repo.as_ref().ok_or(Error::NoRepository)?;
  git_tag::list_tags(repo)
}

#[tauri::command]
pub async fn create_tag(
  name: String,
  message: Option<String>,
  commit: Option<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<TagEntry>> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.create_tag(&name, message.as_deref(), commit.as_deref()).await?;

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let tags = git_tag::list_tags(&repo)?;
  win_state.repo = Some(repo);
  Ok(tags)
}

#[tauri::command]
pub async fn delete_tag(
  name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<TagEntry>> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let cli = GitCli::new(&root);
  cli.delete_tag(&name).await?;

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let tags = git_tag::list_tags(&repo)?;
  win_state.repo = Some(repo);
  Ok(tags)
}

#[tauri::command]
pub async fn push_tag(
  remote: String,
  tag: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.push_tag(&remote, &tag).await?;
  Ok(())
}

#[tauri::command]
pub async fn delete_remote_tag(
  remote: String,
  name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.delete_remote_tag(&remote, &name).await?;
  Ok(())
}
