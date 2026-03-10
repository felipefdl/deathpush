use std::path::Path;
use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::types::RepositoryStatus;
use crate::util::async_command;

fn get_repo_root(state: &State<'_, Mutex<AppRepoState>>, window: &WebviewWindow) -> Result<std::path::PathBuf> {
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
  win_state.cli_root.clone().ok_or(Error::NoRepository)
}

fn resolve_safe_path(root: &Path, relative: &str) -> Result<std::path::PathBuf> {
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let full = root.join(relative);
  let canon = full
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve path: {}", e)))?;
  if !canon.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }
  Ok(canon)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
  std::fs::create_dir_all(dst)?;
  for entry in std::fs::read_dir(src)? {
    let entry = entry?;
    let ty = entry.file_type()?;
    let dest_child = dst.join(entry.file_name());
    if ty.is_dir() {
      copy_dir_recursive(&entry.path(), &dest_child)?;
    } else {
      std::fs::copy(entry.path(), dest_child)?;
    }
  }
  Ok(())
}

fn delete_dir_recursive(path: &Path) -> Result<()> {
  std::fs::remove_dir_all(path)?;
  Ok(())
}

fn generate_copy_name(dest_dir: &Path, original_name: &std::ffi::OsStr) -> std::path::PathBuf {
  let name_str = original_name.to_string_lossy();
  let (stem, ext) = match name_str.rfind('.') {
    Some(idx) if idx > 0 => (&name_str[..idx], Some(&name_str[idx..])),
    _ => (name_str.as_ref(), None),
  };
  let mut counter = 0u32;
  loop {
    let suffix = if counter == 0 {
      " copy".to_string()
    } else {
      format!(" copy {}", counter + 1)
    };
    let new_name = match ext {
      Some(e) => format!("{}{}{}", stem, suffix, e),
      None => format!("{}{}", stem, suffix),
    };
    let path = dest_dir.join(&new_name);
    if !path.exists() {
      return path;
    }
    counter += 1;
    if counter > 999 {
      return path;
    }
  }
}

fn resolve_dest_child(dest: &Path, name: &std::ffi::OsStr, on_conflict: &str) -> Result<std::path::PathBuf> {
  let dest_child = dest.join(name);
  if !dest_child.exists() {
    return Ok(dest_child);
  }
  match on_conflict {
    "replace" => {
      if dest_child.is_dir() {
        delete_dir_recursive(&dest_child)?;
      } else {
        std::fs::remove_file(&dest_child)?;
      }
      Ok(dest_child)
    }
    "keep-both" => Ok(generate_copy_name(dest, name)),
    _ => Err(Error::Other(format!(
      "\"{}\" already exists in destination",
      name.to_string_lossy()
    ))),
  }
}

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
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  let parent = target.parent().ok_or_else(|| Error::Other("Invalid path".into()))?;
  std::fs::create_dir_all(parent)?;
  let canon_parent = parent
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve path: {}", e)))?;
  let file_name = target
    .file_name()
    .ok_or_else(|| Error::Other("Invalid file name".into()))?;
  let full_path = canon_parent.join(file_name);
  if !canon_parent.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }
  std::fs::write(&full_path, content)?;
  Ok(())
}

#[tauri::command]
pub async fn open_in_editor(path: String, state: State<'_, Mutex<AppRepoState>>, window: WebviewWindow) -> Result<()> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };
  let full_path = root.join(&path);

  #[cfg(target_os = "macos")]
  {
    async_command("open").arg(&full_path).output().await?;
  }
  #[cfg(target_os = "linux")]
  {
    async_command("xdg-open").arg(&full_path).output().await?;
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
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let full_path = root
    .join(&path)
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve file path: {}", e)))?;
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
pub async fn delete_files(
  paths: Vec<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let (label, root) = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let label = window.label().to_string();
    let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
    (label, win_state.cli_root.clone().ok_or(Error::NoRepository)?)
  };
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  for rel_path in &paths {
    let full_path = root
      .join(rel_path)
      .canonicalize()
      .map_err(|e| Error::Other(format!("Cannot resolve file path: {}", e)))?;
    if !full_path.starts_with(&canon_root) {
      return Err(Error::Other("Path traversal denied".into()));
    }
    trash::delete(&full_path).map_err(|e| Error::Other(e.to_string()))?;
  }

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

#[tauri::command]
pub async fn rename_entry(
  old_path: String,
  new_name: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  if new_name.is_empty() || new_name.contains('/') || new_name.contains('\\') || new_name.contains('\0') {
    return Err(Error::Other("Invalid file name".into()));
  }
  let root = get_repo_root(&state, &window)?;
  let old_full = resolve_safe_path(&root, &old_path)?;
  let new_full = old_full
    .parent()
    .ok_or_else(|| Error::Other("Invalid path".into()))?
    .join(&new_name);
  if new_full.exists() {
    return Err(Error::Other(format!("\"{}\" already exists", new_name)));
  }
  std::fs::rename(&old_full, &new_full)?;
  Ok(())
}

#[tauri::command]
pub async fn create_directory(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  if path.is_empty() {
    return Err(Error::Other("Path cannot be empty".into()));
  }
  let root = get_repo_root(&state, &window)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  if let Some(parent) = target.parent() {
    std::fs::create_dir_all(parent)?;
    let canon_parent = parent
      .canonicalize()
      .map_err(|e| Error::Other(format!("Cannot resolve path: {}", e)))?;
    if !canon_parent.starts_with(&canon_root) {
      return Err(Error::Other("Path traversal denied".into()));
    }
  }
  std::fs::create_dir_all(&target)?;
  Ok(())
}

#[tauri::command]
pub async fn copy_entries(
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = get_repo_root(&state, &window)?;
  let dest = resolve_safe_path(&root, &destination_dir)?;
  if !dest.is_dir() {
    return Err(Error::Other("Destination is not a directory".into()));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_rel in &sources {
    let src = resolve_safe_path(&root, src_rel)?;
    let name = src
      .file_name()
      .ok_or_else(|| Error::Other("Invalid source path".into()))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
    if src.is_dir() {
      copy_dir_recursive(&src, &dest_child)?;
    } else {
      std::fs::copy(&src, &dest_child)?;
    }
  }
  Ok(())
}

#[tauri::command]
pub async fn move_entries(
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = get_repo_root(&state, &window)?;
  let dest = resolve_safe_path(&root, &destination_dir)?;
  if !dest.is_dir() {
    return Err(Error::Other("Destination is not a directory".into()));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_rel in &sources {
    let src = resolve_safe_path(&root, src_rel)?;
    let name = src
      .file_name()
      .ok_or_else(|| Error::Other("Invalid source path".into()))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
    // Try rename first, fallback to copy + delete for cross-device moves
    if std::fs::rename(&src, &dest_child).is_err() {
      if src.is_dir() {
        copy_dir_recursive(&src, &dest_child)?;
        delete_dir_recursive(&src)?;
      } else {
        std::fs::copy(&src, &dest_child)?;
        std::fs::remove_file(&src)?;
      }
    }
  }
  Ok(())
}

#[tauri::command]
pub async fn duplicate_entry(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<String> {
  let root = get_repo_root(&state, &window)?;
  let src = resolve_safe_path(&root, &path)?;
  let parent = src.parent().ok_or_else(|| Error::Other("Invalid path".into()))?;
  let stem = src
    .file_stem()
    .map(|s| s.to_string_lossy().to_string())
    .unwrap_or_default();
  let ext = src.extension().map(|e| format!(".{}", e.to_string_lossy()));

  let mut new_path;
  let mut counter = 0u32;
  loop {
    let suffix = if counter == 0 {
      " copy".to_string()
    } else {
      format!(" copy {}", counter + 1)
    };
    let new_name = match &ext {
      Some(e) => format!("{}{}{}", stem, suffix, e),
      None => format!("{}{}", stem, suffix),
    };
    new_path = parent.join(&new_name);
    if !new_path.exists() {
      break;
    }
    counter += 1;
    if counter > 999 {
      return Err(Error::Other("Could not generate unique name".into()));
    }
  }

  if src.is_dir() {
    copy_dir_recursive(&src, &new_path)?;
  } else {
    std::fs::copy(&src, &new_path)?;
  }

  let canon_root = root.canonicalize().map_err(|e| Error::Other(e.to_string()))?;
  let relative = new_path
    .strip_prefix(&canon_root)
    .map_err(|_| Error::Other("Cannot compute relative path".into()))?;
  Ok(relative.to_string_lossy().to_string())
}

#[tauri::command]
pub async fn import_files(
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<()> {
  let root = get_repo_root(&state, &window)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let dest = if destination_dir.is_empty() {
    canon_root
  } else {
    resolve_safe_path(&root, &destination_dir)?
  };
  if !dest.is_dir() {
    return Err(Error::Other("Destination is not a directory".into()));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_path in &sources {
    let src = Path::new(src_path);
    if !src.exists() {
      return Err(Error::Other(format!("Source not found: {}", src_path)));
    }
    let name = src
      .file_name()
      .ok_or_else(|| Error::Other("Invalid source path".into()))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
    if src.is_dir() {
      copy_dir_recursive(src, &dest_child)?;
    } else {
      std::fs::copy(src, &dest_child)?;
    }
  }
  Ok(())
}
