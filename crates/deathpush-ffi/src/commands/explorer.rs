use std::fs;
use std::path::Path;

use nucleo_matcher::pattern::{Atom, AtomKind, CaseMatching, Normalization};
use nucleo_matcher::{Config, Matcher, Utf32Str};

use deathpush_core::error::Error;

use deathpush_core::git::diff::{blob_to_data_uri, detect_language, is_image_file};
use deathpush_core::types::{ContentSearchResult, ExplorerEntry, FileContent, FuzzyFileResult};
use deathpush_core::util::async_command;

use crate::session::{get_root, make_cli, manager};

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB
const BINARY_CHECK_SIZE: usize = 8192;

#[uniffi::export]
pub fn list_directory(session_id: String, path: Option<String>) -> Result<Vec<ExplorerEntry>, Error> {
  let root = get_root(&session_id)?;

  let target_dir = match &path {
    Some(p) => root.join(p),
    None => root.clone(),
  };

  if !target_dir.is_dir() {
    return Ok(vec![]);
  }

  let repo = git2::Repository::open(&root).ok();

  let mut entries = Vec::new();
  let read_dir = fs::read_dir(&target_dir)?;

  for entry in read_dir.flatten() {
    let file_name = entry.file_name();
    let name = file_name.to_string_lossy().to_string();

    if name == ".git" {
      continue;
    }

    let entry_path = entry.path();
    let relative = entry_path
      .strip_prefix(&root)
      .unwrap_or(&entry_path)
      .to_string_lossy()
      .to_string();

    if let Some(ref r) = repo {
      if r.status_should_ignore(Path::new(&relative)).unwrap_or(false) {
        continue;
      }
    }

    let metadata = entry.metadata();
    let is_directory = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
    let is_symlink = metadata.as_ref().map(|m| m.is_symlink()).unwrap_or(false);

    entries.push(ExplorerEntry {
      name,
      path: relative,
      is_directory,
      is_symlink,
    });
  }

  entries.sort_by(|a, b| match (a.is_directory, b.is_directory) {
    (true, false) => std::cmp::Ordering::Less,
    (false, true) => std::cmp::Ordering::Greater,
    _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
  });

  Ok(entries)
}

#[uniffi::export]
pub fn read_file_content(session_id: String, path: String) -> Result<FileContent, Error> {
  let root = get_root(&session_id)?;

  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve repository root: {}", e)))?;
  let target = root.join(&path);
  let canon_target = target
    .canonicalize()
    .map_err(|e| Error::other(format!("Cannot resolve file path: {}", e)))?;
  if !canon_target.starts_with(&canon_root) {
    return Err(Error::other("Path traversal denied"));
  }

  if !canon_target.is_file() {
    return Err(Error::other("File not found"));
  }

  let metadata = fs::metadata(&canon_target)?;
  if metadata.len() > MAX_FILE_SIZE {
    return Ok(FileContent {
      path,
      content: String::new(),
      language: None,
      file_type: "large".to_string(),
    });
  }

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

  let bytes = fs::read(&canon_target)?;
  let check_len = bytes.len().min(BINARY_CHECK_SIZE);
  if bytes[..check_len].contains(&0) {
    return Ok(FileContent {
      path,
      content: String::new(),
      language: None,
      file_type: "binary".to_string(),
    });
  }

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

#[uniffi::export]
pub fn fuzzy_find_files(session_id: String, query: String, max_results: u32) -> Result<Vec<FuzzyFileResult>, Error> {
  let root = get_root(&session_id)?;
  let max = max_results as usize;

  let cli = make_cli(&root);
  let output = manager()
    .runtime
    .block_on(cli.run(&["ls-files", "--cached", "--others", "--exclude-standard"]))?;

  let files: Vec<&str> = output.lines().filter(|l| !l.is_empty()).collect();

  if query.is_empty() {
    let mut results: Vec<FuzzyFileResult> = files
      .into_iter()
      .take(max)
      .map(|path| FuzzyFileResult {
        path: path.to_string(),
        score: 0,
        match_positions: vec![],
      })
      .collect();
    results.sort_by(|a, b| a.path.to_lowercase().cmp(&b.path.to_lowercase()));
    return Ok(results.into_iter().take(max).collect());
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
        match_positions: indices.to_vec(),
      });
    }
    buf.clear();
  }

  scored.sort_by(|a, b| b.score.cmp(&a.score));
  scored.truncate(max);
  Ok(scored)
}

#[uniffi::export]
pub fn search_file_contents(
  session_id: String,
  query: String,
  max_results: u32,
) -> Result<Vec<ContentSearchResult>, Error> {
  if query.is_empty() {
    return Ok(vec![]);
  }

  let root = get_root(&session_id)?;
  let max = max_results as usize;

  let output = manager().runtime.block_on(async {
    async_command("git")
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
      .map_err(|e| Error::other(e.to_string()))
  })?;

  if !output.status.success() {
    let code = output.status.code().unwrap_or(-1);
    if code == 1 {
      return Ok(vec![]);
    }
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    return Err(Error::git_cli(stderr));
  }

  let stdout = String::from_utf8_lossy(&output.stdout);
  let mut results = Vec::new();

  for line in stdout.lines() {
    if results.len() >= max {
      break;
    }
    let Some((path, rest)) = line.split_once(':') else {
      continue;
    };
    let Some((line_num_str, rest)) = rest.split_once(':') else {
      continue;
    };
    let Some((col_str, content)) = rest.split_once(':') else {
      continue;
    };
    let Ok(line_number) = line_num_str.parse::<u32>() else {
      continue;
    };
    let Ok(column) = col_str.parse::<u32>() else {
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
