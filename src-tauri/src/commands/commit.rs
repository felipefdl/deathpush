use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::types::RepositoryStatus;

#[tauri::command]
pub async fn commit(
  message: String,
  amend: bool,
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
  cli.commit(&message, amend).await?;

  let mut guard = state.lock().map_err(|e| Error::other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  win_state.repo = Some(repo);
  Ok(status)
}
