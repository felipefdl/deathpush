pub mod blame;
pub mod branch;
pub mod cli;
pub mod commit;
pub mod config;
pub mod file_ops;
pub mod lifecycle;
pub mod log;
pub mod remote;
pub mod repository;
pub mod staging;
pub mod stash;
pub mod status;
pub mod tag;
pub mod terminal;

use tauri::WebviewWindow;

use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::types::RepositoryStatus;

use self::repository::AppRepoState;

pub fn update_window_title(window: &WebviewWindow, status: &RepositoryStatus) {
  let repo_name = std::path::Path::new(&status.root)
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "DeathPush".into());
  let branch = status.head_branch.as_deref().unwrap_or("");
  let title = if branch.is_empty() {
    format!("{} - DeathPush", repo_name)
  } else {
    format!("{} ({}) - DeathPush", repo_name, branch)
  };
  let _ = window.set_title(&title);
}

pub fn refresh_status(app_state: &mut AppRepoState, label: &str) -> Result<RepositoryStatus> {
  let win_state = app_state.windows.get_mut(label).ok_or(Error::NoRepository)?;
  let root = win_state.cli_root.as_ref().ok_or(Error::NoRepository)?.clone();
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  win_state.repo = Some(repo);
  Ok(status)
}
