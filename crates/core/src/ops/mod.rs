pub mod cli;
pub mod config;
pub mod explorer;
pub mod file_ops;
pub mod repository;
pub mod session;
pub mod terminal;

use std::sync::Mutex;

use tauri::{Manager, WebviewWindow};

use crate::error::{Error, Result};
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::git::status::StatusScope;

use self::repository::AppRepoState;

pub fn update_window_title(window: &WebviewWindow, root: &str, head_branch: Option<&str>) {
  let repo_name = std::path::Path::new(root)
    .file_name()
    .map(|n| n.to_string_lossy().to_string())
    .unwrap_or_else(|| "DeathPush".into());
  let branch = head_branch.unwrap_or("");
  let title = if branch.is_empty() {
    format!("{} - DeathPush", repo_name)
  } else {
    format!("{} ({}) - DeathPush", repo_name, branch)
  };
  let _ = window.set_title(&title);
}

pub fn invalidate_status(app_state: &Mutex<AppRepoState>, window: &WebviewWindow) -> Result<()> {
  invalidate_status_with(app_state, window, |runtime| {
    runtime.invalidate(StatusScope::Repository);
    Ok(())
  })
}

pub fn invalidate_status_paths(
  app_state: &Mutex<AppRepoState>,
  window: &WebviewWindow,
  paths: &[String],
) -> Result<()> {
  invalidate_status_with(app_state, window, |runtime| {
    runtime.invalidate_paths(paths);
    Ok(())
  })
}

fn invalidate_status_with(
  app_state: &Mutex<AppRepoState>,
  window: &WebviewWindow,
  invalidate: impl FnOnce(&crate::git::repository_runtime::RepositoryRuntime) -> Result<()>,
) -> Result<()> {
  let label = window.label();
  let runtime = window
    .state::<RepositoryRuntimeRegistry>()
    .runtime_for_window(label)
    .ok_or(Error::NoRepository)?;
  invalidate(&runtime)?;
  let repo = runtime.open_repository()?;
  update_window_title(window, &repo.root().to_string_lossy(), repo.head_branch().as_deref());

  let mut app_state = app_state.lock().map_err(|err| Error::Other(err.to_string()))?;
  let win_state = app_state.windows.get_mut(label).ok_or(Error::NoRepository)?;
  win_state.repo = Some(repo);
  Ok(())
}
