use std::fs;
use std::path::Path;
use std::sync::Mutex;

use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use tauri::{State, WebviewWindow};

use crate::commands::repository::AppRepoState;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::diff::{blob_to_data_uri, detect_language, is_image_file};
use crate::types::{ContentSearchResult, ExplorerEntry, FileContent, FuzzyFileResult};
use crate::util::async_command_ready;

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB
const BINARY_CHECK_SIZE: usize = 8192;

async fn collect_repository_entries(root: &Path) -> Result<Vec<ExplorerEntry>> {
  let output = GitCli::new(root)
    .run(&["ls-files", "-z", "--cached", "--others", "--exclude-standard"])
    .await?;
  let mut entries = output
    .split('\0')
    .filter(|path| !path.is_empty())
    .map(|path| {
      let path = Path::new(path);
      let relative_path = path.to_string_lossy().to_string();
      let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| relative_path.clone());
      let is_symlink = fs::symlink_metadata(root.join(path))
        .map(|metadata| metadata.file_type().is_symlink())
        .unwrap_or(false);
      ExplorerEntry {
        name,
        path: relative_path,
        is_directory: false,
        is_symlink,
      }
    })
    .collect::<Vec<_>>();
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
        path,
        content,
        language,
        file_type: "text".to_string(),
      })
    }
    Err(_) => Ok(FileContent {
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
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<FuzzyFileResult>> {
  let root = {
    let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
    let win_state = guard.get(window.label()).ok_or(Error::NoRepository)?;
    win_state.cli_root.clone().ok_or(Error::NoRepository)?
  };

  let cli = GitCli::new(&root);
  let output = cli
    .run(&["ls-files", "--cached", "--others", "--exclude-standard"])
    .await?;

  let files: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();

  if query.is_empty() {
    let mut results: Vec<FuzzyFileResult> = files
      .into_iter()
      .take(max_results)
      .map(|path| FuzzyFileResult {
        path: path.to_string(),
        score: 0,
        match_positions: vec![],
      })
      .collect();
    results.sort_by_key(|result| result.path.to_lowercase());
    return Ok(results.into_iter().take(max_results).collect());
  }

  let mut matcher = Matcher::new(Config::DEFAULT.match_paths());
  let atom = Atom::new(
    &query,
    CaseMatching::Ignore,
    Normalization::Smart,
    AtomKind::Fuzzy,
    false,
  );

  let mut scored: Vec<FuzzyFileResult> = Vec::new();
  let mut buf = Vec::new();

  for file_path in &files {
    let mut indices = Vec::new();
    let haystack = Utf32Str::new(file_path, &mut buf);
    if let Some(score) = atom.indices(haystack, &mut matcher, &mut indices) {
      scored.push(FuzzyFileResult {
        path: file_path.to_string(),
        score: score as u32,
        match_positions: indices.iter().map(|&i| i as usize).collect(),
      });
    }
    buf.clear();
  }

  scored.sort_by_key(|result| std::cmp::Reverse(result.score));
  scored.truncate(max_results);
  Ok(scored)
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
  async fn repository_tree_uses_git_file_listing_and_skips_ignored_paths() {
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
    fs::write(root.join(".gitignore"), "target/\n").expect("gitignore should be created");
    git2::Repository::init(&root).expect("repository should be initialized");

    let entries = collect_repository_entries(&root)
      .await
      .expect("repository tree should be collected");
    let paths = entries
      .iter()
      .map(|entry| (entry.path.as_str(), entry.is_directory))
      .collect::<Vec<_>>();

    assert_eq!(paths, vec![(".gitignore", false), ("src/index.ts", false)]);

    fs::remove_dir_all(root).expect("temporary repository should be removed");
  }
}
