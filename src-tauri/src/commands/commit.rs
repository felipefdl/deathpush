use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::invalidate_status;
use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;

#[tauri::command]
pub async fn commit(
  message: String,
  amend: bool,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let cli = GitCli::new(&root);
  cli.commit(&message, amend).await?;
  invalidate_status(state.inner(), &window)
}
