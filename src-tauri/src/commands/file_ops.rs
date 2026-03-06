use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::types::RepositoryStatus;

#[tauri::command]
pub async fn write_file(
  path: String,
  content: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let full_path = root.join(&path).canonicalize().unwrap_or_else(|_| root.join(&path));
  let canon_root = root.canonicalize().unwrap_or(root);
  if !full_path.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }
  std::fs::write(&full_path, content)?;
  Ok(())
}

#[tauri::command]
pub async fn open_in_editor(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let full_path = root.join(&path);

  #[cfg(target_os = "macos")]
  {
    tokio::process::Command::new("open")
      .arg(&full_path)
      .output()
      .await?;
  }
  #[cfg(target_os = "linux")]
  {
    tokio::process::Command::new("xdg-open")
      .arg(&full_path)
      .output()
      .await?;
  }
  #[cfg(target_os = "windows")]
  {
    tokio::process::Command::new("cmd")
      .args(["/c", "start", "", &full_path.to_string_lossy()])
      .output()
      .await?;
  }

  Ok(())
}

#[tauri::command]
pub async fn reveal_in_file_manager(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let full_path = root.join(&path);

  #[cfg(target_os = "macos")]
  {
    tokio::process::Command::new("open")
      .args(["-R", &full_path.to_string_lossy()])
      .output()
      .await?;
  }
  #[cfg(target_os = "linux")]
  {
    tokio::process::Command::new("xdg-open")
      .arg(full_path.parent().unwrap_or(&full_path))
      .output()
      .await?;
  }
  #[cfg(target_os = "windows")]
  {
    tokio::process::Command::new("explorer")
      .args(["/select,", &full_path.to_string_lossy()])
      .output()
      .await?;
  }

  Ok(())
}

#[tauri::command]
pub async fn delete_file(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let full_path = root.join(&path).canonicalize().unwrap_or_else(|_| root.join(&path));
  let canon_root = root.canonicalize().unwrap_or(root.clone());
  if !full_path.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }
  trash::delete(&full_path).map_err(|e| Error::Other(e.to_string()))?;

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  win_state.repo = Some(repo);
  Ok(status)
}

#[tauri::command]
pub async fn add_to_gitignore(
  pattern: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let gitignore_path = root.join(".gitignore");
  let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
  if !content.ends_with('\n') && !content.is_empty() {
    content.push('\n');
  }
  content.push_str(&pattern);
  content.push('\n');
  std::fs::write(&gitignore_path, content)?;

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;
  win_state.repo = Some(repo);
  Ok(status)
}
