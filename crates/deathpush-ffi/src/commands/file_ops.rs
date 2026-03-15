use std::path::Path;

use deathpush_core::error::Error;
use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::types::RepositoryStatus;
use deathpush_core::util::async_command;

use crate::session::{get_root, manager};

fn resolve_safe_path(root: &Path, relative: &str) -> Result<std::path::PathBuf, Error> {
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let full = root.join(relative);
  let canon = full
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve path: {}", e)))?;
  if !canon.starts_with(&canon_root) {
    return Err(Error::other("Path traversal denied"));
  }
  Ok(canon)
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), Error> {
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

fn delete_dir_recursive(path: &Path) -> Result<(), Error> {
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

fn resolve_dest_child(dest: &Path, name: &std::ffi::OsStr, on_conflict: &str) -> Result<std::path::PathBuf, Error> {
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
    _ => Err(Error::other(format!(
      "\"{}\" already exists in destination",
      name.to_string_lossy()
    ))),
  }
}

fn refresh_with_repo(session_id: &str) -> Result<RepositoryStatus, Error> {
  let root = get_root(session_id)?;
  let repo = GitRepository::open(&root)?;
  let status = get_repository_status(&repo)?;

  let mut sessions = manager().sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  if let Some(state) = sessions.get_mut(session_id) {
    state.repo = Some(repo);
  }

  Ok(status)
}

#[uniffi::export]
pub fn write_file(session_id: String, path: String, content: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  let parent = target.parent().ok_or_else(|| Error::other("Invalid path"))?;
  std::fs::create_dir_all(parent)?;
  let canon_parent = parent
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve path: {}", e)))?;
  let file_name = target.file_name().ok_or_else(|| Error::other("Invalid file name"))?;
  let full_path = canon_parent.join(file_name);
  if !canon_parent.starts_with(&canon_root) {
    return Err(Error::other("Path traversal denied"));
  }
  std::fs::write(&full_path, content)?;
  Ok(())
}

#[uniffi::export]
pub fn open_in_editor(session_id: String, path: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let full_path = root.join(&path);

  manager().runtime.block_on(async {
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
    Ok::<_, Error>(())
  })?;

  Ok(())
}

#[uniffi::export]
pub fn reveal_in_file_manager(session_id: String, path: String) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let full_path = root.join(&path);

  manager().runtime.block_on(async {
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
    Ok::<_, Error>(())
  })?;

  Ok(())
}

#[uniffi::export]
pub fn delete_file(session_id: String, path: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let full_path = root
    .join(&path)
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve file path: {}", e)))?;
  if !full_path.starts_with(&canon_root) {
    return Err(Error::other("Path traversal denied"));
  }
  trash::delete(&full_path).map_err(|e| Error::other(e.to_string()))?;
  refresh_with_repo(&session_id)
}

#[uniffi::export]
pub fn delete_files(session_id: String, paths: Vec<String>) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  for rel_path in &paths {
    let full_path = root
      .join(rel_path)
      .canonicalize()
      .map_err(|e| Error::other(format!("Cannot resolve file path: {}", e)))?;
    if !full_path.starts_with(&canon_root) {
      return Err(Error::other("Path traversal denied"));
    }
    trash::delete(&full_path).map_err(|e| Error::other(e.to_string()))?;
  }
  refresh_with_repo(&session_id)
}

#[uniffi::export]
pub fn add_to_gitignore(session_id: String, pattern: String) -> Result<RepositoryStatus, Error> {
  let root = get_root(&session_id)?;
  let gitignore_path = root.join(".gitignore");
  let mut content = std::fs::read_to_string(&gitignore_path).unwrap_or_default();
  if !content.ends_with('\n') && !content.is_empty() {
    content.push('\n');
  }
  content.push_str(&pattern);
  content.push('\n');
  std::fs::write(&gitignore_path, content)?;
  refresh_with_repo(&session_id)
}

#[uniffi::export]
pub fn rename_entry(session_id: String, old_path: String, new_name: String) -> Result<(), Error> {
  if new_name.is_empty() || new_name.contains('/') || new_name.contains('\\') || new_name.contains('\0') {
    return Err(Error::other("Invalid file name"));
  }
  let root = get_root(&session_id)?;
  let old_full = resolve_safe_path(&root, &old_path)?;
  let new_full = old_full
    .parent()
    .ok_or_else(|| Error::other("Invalid path"))?
    .join(&new_name);
  if new_full.exists() {
    return Err(Error::other(format!("\"{}\" already exists", new_name)));
  }
  std::fs::rename(&old_full, &new_full)?;
  Ok(())
}

#[uniffi::export]
pub fn create_directory(session_id: String, path: String) -> Result<(), Error> {
  if path.is_empty() {
    return Err(Error::other("Path cannot be empty"));
  }
  let root = get_root(&session_id)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  if let Some(parent) = target.parent() {
    std::fs::create_dir_all(parent)?;
    let canon_parent = parent
      .canonicalize()
      .map_err(|e| Error::other(format!("Cannot resolve path: {}", e)))?;
    if !canon_parent.starts_with(&canon_root) {
      return Err(Error::other("Path traversal denied"));
    }
  }
  std::fs::create_dir_all(&target)?;
  Ok(())
}

#[uniffi::export]
pub fn copy_entries(
  session_id: String,
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let dest = resolve_safe_path(&root, &destination_dir)?;
  if !dest.is_dir() {
    return Err(Error::other("Destination is not a directory"));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_rel in &sources {
    let src = resolve_safe_path(&root, src_rel)?;
    let name = src.file_name().ok_or_else(|| Error::other("Invalid source path"))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
    if src.is_dir() {
      copy_dir_recursive(&src, &dest_child)?;
    } else {
      std::fs::copy(&src, &dest_child)?;
    }
  }
  Ok(())
}

#[uniffi::export]
pub fn move_entries(
  session_id: String,
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let dest = resolve_safe_path(&root, &destination_dir)?;
  if !dest.is_dir() {
    return Err(Error::other("Destination is not a directory"));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_rel in &sources {
    let src = resolve_safe_path(&root, src_rel)?;
    let name = src.file_name().ok_or_else(|| Error::other("Invalid source path"))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
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

#[uniffi::export]
pub fn duplicate_entry(session_id: String, path: String) -> Result<String, Error> {
  let root = get_root(&session_id)?;
  let src = resolve_safe_path(&root, &path)?;
  let parent = src.parent().ok_or_else(|| Error::other("Invalid path"))?;
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
      return Err(Error::other("Could not generate unique name"));
    }
  }

  if src.is_dir() {
    copy_dir_recursive(&src, &new_path)?;
  } else {
    std::fs::copy(&src, &new_path)?;
  }

  let canon_root = root.canonicalize().map_err(|e| Error::other(e.to_string()))?;
  let relative = new_path
    .strip_prefix(&canon_root)
    .map_err(|_| Error::other("Cannot compute relative path"))?;
  Ok(relative.to_string_lossy().to_string())
}

#[uniffi::export]
pub fn import_files(
  session_id: String,
  sources: Vec<String>,
  destination_dir: String,
  on_conflict: Option<String>,
) -> Result<(), Error> {
  let root = get_root(&session_id)?;
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let dest = if destination_dir.is_empty() {
    canon_root
  } else {
    resolve_safe_path(&root, &destination_dir)?
  };
  if !dest.is_dir() {
    return Err(Error::other("Destination is not a directory"));
  }
  let conflict = on_conflict.as_deref().unwrap_or("error");
  for src_path in &sources {
    let src = Path::new(src_path);
    if !src.exists() {
      return Err(Error::other(format!("Source not found: {}", src_path)));
    }
    let name = src.file_name().ok_or_else(|| Error::other("Invalid source path"))?;
    let dest_child = resolve_dest_child(&dest, name, conflict)?;
    if src.is_dir() {
      copy_dir_recursive(src, &dest_child)?;
    } else {
      std::fs::copy(src, &dest_child)?;
    }
  }
  Ok(())
}
