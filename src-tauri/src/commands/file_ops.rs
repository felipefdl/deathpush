use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::util::async_command;
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
  let canon_root = root.canonicalize().map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  let parent = target.parent().ok_or_else(|| Error::Other("Invalid path".into()))?;
  std::fs::create_dir_all(parent)?;
  let canon_parent = parent.canonicalize().map_err(|e| Error::Other(format!("Cannot resolve path: {}", e)))?;
  let file_name = target.file_name().ok_or_else(|| Error::Other("Invalid file name".into()))?;
  let full_path = canon_parent.join(file_name);
  if !canon_parent.starts_with(&canon_root) {
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
    async_command("open")
      .arg(&full_path)
      .output()
      .await?;
  }
  #[cfg(target_os = "linux")]
  {
    async_command("xdg-open")
      .arg(&full_path)
      .output()
      .await?;
  }
  #[cfg(target_os = "windows")]
  {
    async_command("cmd")
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
    async_command("open")
      .args(["-R", &full_path.to_string_lossy()])
      .output()
      .await?;
  }
  #[cfg(target_os = "linux")]
  {
    async_command("xdg-open")
      .arg(full_path.parent().unwrap_or(&full_path))
      .output()
      .await?;
  }
  #[cfg(target_os = "windows")]
  {
    async_command("explorer")
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
  let canon_root = root.canonicalize().map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let full_path = root.join(&path).canonicalize().map_err(|e| Error::Other(format!("Cannot resolve file path: {}", e)))?;
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
