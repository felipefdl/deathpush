use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::Mutex;

use tauri::{Manager, State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::diff::{blob_to_data_uri, detect_language, is_image_file};
use crate::git::repository_runtime::RepositoryRuntimeRegistry;
use crate::types::{ContentSearchResult, ExplorerEntry, FileContent, FuzzyFileResult};
use crate::util::async_command_ready;

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB
const BINARY_CHECK_SIZE: usize = 8192;

fn is_hard_hidden(path: &str) -> bool {
  path
    .split(['/', '\\'])
    .any(|part| matches!(part, ".git" | ".svn" | ".hg" | ".DS_Store" | "Thumbs.db"))
}

fn parse_listed_path(raw: &str) -> Option<(String, bool)> {
  if raw.is_empty() {
    return None;
  }
  let is_directory = raw.ends_with('/') || raw.ends_with('\\');
  let path = raw.trim_end_matches(['/', '\\']).replace('\\', "/");
  if path.is_empty() || is_hard_hidden(&path) {
    return None;
  }
  Some((path, is_directory))
}

fn explorer_entry(root: &Path, path: String, is_directory: bool, ignored: bool) -> ExplorerEntry {
  let name = Path::new(&path)
    .file_name()
    .map(|name| name.to_string_lossy().to_string())
    .unwrap_or_else(|| path.clone());
  let is_symlink = fs::symlink_metadata(root.join(&path))
    .map(|metadata| metadata.file_type().is_symlink())
    .unwrap_or(false);
  ExplorerEntry {
    name,
    path,
    is_directory,
    is_symlink,
    ignored,
  }
}

fn push_listed_paths(
  root: &Path,
  output: &str,
  ignored: bool,
  seen: &mut HashSet<String>,
  entries: &mut Vec<ExplorerEntry>,
) {
  for raw in output.split('\0') {
    let Some((path, is_directory)) = parse_listed_path(raw) else {
      continue;
    };
    if !seen.insert(path.clone()) {
      continue;
    }
    entries.push(explorer_entry(root, path, is_directory, ignored));
  }
}

async fn collect_repository_entries(root: &Path) -> Result<Vec<ExplorerEntry>> {
  let cli = GitCli::new(root);
  let visible = cli
    .run(&["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
    .await?;
  let ignored = cli
    .run(&[
      "ls-files",
      "-z",
      "--others",
      "--ignored",
      "--exclude-standard",
      "--directory",
    ])
    .await?;

  let mut seen = HashSet::new();
  let mut entries = Vec::new();
  push_listed_paths(root, &visible, false, &mut seen, &mut entries);
  push_listed_paths(root, &ignored, true, &mut seen, &mut entries);
  entries.sort_by_cached_key(|entry| entry.path.to_lowercase());
  Ok(entries)
}

fn collect_directory_entries(root: &Path, relative: &str) -> Result<Vec<ExplorerEntry>> {
  let relative = relative.trim_end_matches(['/', '\\']).replace('\\', "/");
  if relative
    .split('/')
    .any(|part| part.is_empty() || part == "." || part == "..")
    || is_hard_hidden(&relative)
  {
    return Ok(Vec::new());
  }
  let dir = root.join(&relative);
  let read = match fs::read_dir(&dir) {
    Ok(read) => read,
    Err(_) => return Ok(Vec::new()),
  };
  let repo = git2::Repository::open(root).ok();
  let mut entries = Vec::new();
  for child in read.flatten() {
    let name = child.file_name();
    let name = name.to_string_lossy();
    if name == "." || name == ".." || is_hard_hidden(name.as_ref()) {
      continue;
    }
    let path = format!("{relative}/{name}");
    if is_hard_hidden(&path) {
      continue;
    }
    let file_type = child.file_type().ok();
    let is_directory = file_type.is_some_and(|kind| kind.is_dir());
    let is_symlink = file_type.is_some_and(|kind| kind.is_symlink());
    let ignored = repo
      .as_ref()
      .map(|repo| {
        repo.is_path_ignored(Path::new(&path)).unwrap_or(false)
          || (is_directory && repo.is_path_ignored(Path::new(&format!("{path}/"))).unwrap_or(false))
      })
      .unwrap_or(false);
    entries.push(ExplorerEntry {
      name: name.to_string(),
      path,
      is_directory,
      is_symlink,
      ignored,
    });
  }
  entries.sort_by_cached_key(|entry| entry.path.to_lowercase());
  Ok(entries)
}

#[tauri::command]
pub async fn list_repository_tree(
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<ExplorerEntry>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let window_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    window_state.cli_root.clone().ok_or(Error::NoRepository)?
  };

  collect_repository_entries(&root).await
}

#[tauri::command]
pub async fn list_repository_children(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<ExplorerEntry>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let window_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    window_state.cli_root.clone().ok_or(Error::NoRepository)?
  };

  collect_directory_entries(&root, &path)
}

#[tauri::command]
pub async fn read_file_content(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<FileContent> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };

  // Path traversal protection
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  let canon_target = target
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve file path: {}", e)))?;
  if !canon_target.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }

  // Check file exists
  if !canon_target.is_file() {
    return Err(Error::Other("File not found".into()));
  }

  // Size check
  let metadata = fs::metadata(&canon_target)?;
  if metadata.len() > MAX_FILE_SIZE {
    return Ok(FileContent {
      content_hash: crate::content_hash::sha256_utf8(""),
      path,
      content: String::new(),
      language: None,
      file_type: "large".to_string(),
    });
  }

  // Image files
  if is_image_file(&path) {
    let bytes = fs::read(&canon_target)?;
    let data_uri = blob_to_data_uri(&bytes, &path);
    return Ok(FileContent {
      content_hash: crate::content_hash::sha256_utf8(&data_uri),
      path,
      content: data_uri,
      language: None,
      file_type: "image".to_string(),
    });
  }

  // Read raw bytes for binary detection
  let bytes = fs::read(&canon_target)?;

  // Binary detection: check for null bytes in first 8KB
  let check_len = bytes.len().min(BINARY_CHECK_SIZE);
  if bytes[..check_len].contains(&0) {
    return Ok(FileContent {
      content_hash: crate::content_hash::sha256_utf8(""),
      path,
      content: String::new(),
      language: None,
      file_type: "binary".to_string(),
    });
  }

  // Try UTF-8 conversion
  match String::from_utf8(bytes) {
    Ok(content) => {
      let language = detect_language(&path);
      Ok(FileContent {
        content_hash: crate::content_hash::sha256_utf8(&content),
        path,
        content,
        language,
        file_type: "text".to_string(),
      })
    }
    Err(_) => Ok(FileContent {
      content_hash: crate::content_hash::sha256_utf8(""),
      path,
      content: String::new(),
      language: None,
      file_type: "binary".to_string(),
    }),
  }
}

#[tauri::command]
pub async fn fuzzy_find_files(
  query: String,
  max_results: usize,
  window: WebviewWindow,
) -> Result<Vec<FuzzyFileResult>> {
  let runtime = window
    .state::<RepositoryRuntimeRegistry>()
    .runtime_for_window(window.label())
    .ok_or(Error::NoRepository)?;
  runtime.fuzzy_find(&query, max_results)
}

#[tauri::command]
pub async fn search_file_contents(
  query: String,
  max_results: usize,
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<ContentSearchResult>> {
  if query.is_empty() {
    return Ok(vec![]);
  }

  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };

  let output = async_command_ready("git")
    .await
    .args([
      "grep",
      "-n",
      "--column",
      "-I",
      "-F",
      "--no-recurse-submodules",
      "--untracked",
      "-e",
      &query,
      "--",
      ".",
    ])
    .current_dir(&root)
    .output()
    .await
    .map_err(|e| Error::Other(e.to_string()))?;

  // git grep exits 1 when no matches found -- not an error
  if !output.status.success() {
    let code = output.status.code().unwrap_or(-1);
    if code == 1 {
      return Ok(vec![]);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::GitCli(stderr));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut results = Vec::new();

  for line in stdout.lines() {
    if results.len() >= max_results {
      break;
    }
    // Format: file:linenum:column:content
    let Some((path, rest)) = line.split_once(':') else {
      continue;
    };
    let Some((line_num_str, rest)) = rest.split_once(':') else {
      continue;
    };
    let Some((col_str, content)) = rest.split_once(':') else {
      continue;
    };
    let Ok(line_number) = line_num_str.parse::<usize>() else {
      continue;
    };
    let Ok(column) = col_str.parse::<usize>() else {
      continue;
    };
    results.push(ContentSearchResult {
      path: path.to_string(),
      line_number,
      column,
      line_content: content.to_string(),
    });
  }

  Ok(results)
}

#[cfg(test)]
mod tests {
  use std::time::{SystemTime, UNIX_EPOCH};

  use super::*;

  #[tokio::test]
  async fn repository_tree_shows_gitignored_paths_and_hides_git_metadata() {
    let suffix = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock must be after the Unix epoch")
      .as_nanos();
    let root = std::env::temp_dir().join(format!("deathpush-explorer-{}-{suffix}", std::process::id()));

    fs::create_dir_all(root.join("src")).expect("src directory should be created");
    fs::create_dir_all(root.join("empty")).expect("empty directory should be created");
    fs::create_dir_all(root.join("target")).expect("ignored directory should be created");
    fs::write(root.join("src/index.ts"), "").expect("source file should be created");
    fs::write(root.join("target/output.js"), "").expect("ignored file should be created");
    fs::write(root.join("noise.log"), "").expect("ignored file should be created");
    fs::write(root.join(".DS_Store"), "").expect("finder metadata should be created");
    fs::write(root.join(".gitignore"), "target/\n*.log\n").expect("gitignore should be created");
    git2::Repository::init(&root).expect("repository should be initialized");

    let entries = collect_repository_entries(&root)
      .await
      .expect("repository tree should be collected");
    let paths = entries
      .iter()
      .map(|entry| (entry.path.as_str(), entry.is_directory, entry.ignored))
      .collect::<Vec<_>>();

    assert_eq!(
      paths,
      vec![
        (".gitignore", false, false),
        ("noise.log", false, true),
        ("src/index.ts", false, false),
        ("target", true, true),
      ]
    );
    assert!(entries.iter().all(|entry| {
      !entry
        .path
        .split('/')
        .any(|part| matches!(part, ".git" | ".svn" | ".hg" | ".DS_Store" | "Thumbs.db"))
    }));

    fs::remove_dir_all(root).expect("temporary repository should be removed");
  }

  #[test]
  fn directory_listing_returns_ignored_children_and_hides_metadata() {
    let suffix = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .expect("clock must be after the Unix epoch")
      .as_nanos();
    let root = std::env::temp_dir().join(format!("deathpush-explorer-children-{}-{suffix}", std::process::id()));

    fs::create_dir_all(root.join("target/nested")).expect("ignored directory should be created");
    fs::write(root.join("target/output.js"), "").expect("ignored file should be created");
    fs::write(root.join("target/.DS_Store"), "").expect("finder metadata should be created");
    fs::write(root.join(".gitignore"), "target/\n").expect("gitignore should be created");
    git2::Repository::init(&root).expect("repository should be initialized");

    let entries = collect_directory_entries(&root, "target").expect("ignored directory should be listed");
    let paths = entries
      .iter()
      .map(|entry| (entry.path.as_str(), entry.is_directory, entry.ignored))
      .collect::<Vec<_>>();

    assert_eq!(
      paths,
      vec![("target/nested", true, true), ("target/output.js", false, true)]
    );

    fs::remove_dir_all(root).expect("temporary repository should be removed");
  }
}
