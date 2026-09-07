use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

use crate::core::Core;
use crate::error::{Error, Result};
use crate::git::cli::GitCli;
use crate::git::diff::{detect_language, is_image_file, is_pdf_file};
use crate::git::repository::GitRepository;
use crate::pdf::PdfPageImage;
use crate::session::SessionId;
use crate::types::{ContentSearchResult, ExplorerEntry, FileContent, FuzzyFileResult};
use crate::util::async_command_ready;

const MAX_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5 MiB of text is already unreadable
const MAX_IMAGE_SIZE: u64 = 64 * 1024 * 1024;
const MAX_PDF_SIZE: u64 = 128 * 1024 * 1024;
const BINARY_CHECK_SIZE: usize = 8192;

/// How `read_file_content` should treat a file. Extension decides first: a 40 MB screenshot is
/// still an image, and the text budget has nothing to say about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReadPlan {
  Oversized,
  Pdf,
  Image,
  Text,
}

fn read_plan(path: &str, len: u64) -> ReadPlan {
  if is_pdf_file(path) {
    return if len > MAX_PDF_SIZE {
      ReadPlan::Oversized
    } else {
      ReadPlan::Pdf
    };
  }
  if is_image_file(path) {
    return if len > MAX_IMAGE_SIZE {
      ReadPlan::Oversized
    } else {
      ReadPlan::Image
    };
  }
  if len > MAX_FILE_SIZE {
    return ReadPlan::Oversized;
  }
  ReadPlan::Text
}

/// Canonicalize `path` inside `root` and refuse anything that escapes it.
fn resolve_repo_file(root: &Path, path: &str) -> Result<PathBuf> {
  let canon_root = root
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve repository root: {}", e)))?;
  let canon_target = root
    .join(path)
    .canonicalize()
    .map_err(|e| Error::Other(format!("Cannot resolve file path: {}", e)))?;
  if !canon_target.starts_with(&canon_root) {
    return Err(Error::Other("Path traversal denied".into()));
  }
  if !canon_target.is_file() {
    return Err(Error::Other("File not found".into()));
  }
  Ok(canon_target)
}

fn modified_nanos(metadata: &fs::Metadata) -> u128 {
  metadata
    .modified()
    .ok()
    .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
    .map(|since| since.as_nanos())
    .unwrap_or(0)
}

fn placeholder(path: &str, file_type: &str) -> FileContent {
  FileContent {
    content_hash: crate::content_hash::sha256_utf8(""),
    path: path.to_string(),
    content: String::new(),
    bytes: None,
    language: None,
    file_type: file_type.to_string(),
  }
}

fn oversized(path: &str) -> FileContent {
  placeholder(path, "large")
}

fn binary(path: &str) -> FileContent {
  placeholder(path, "binary")
}

/// `New File`, `New File 2`, `New File 3`... skipping names already present (case-sensitive) in `existing`.
pub fn next_entry_name(existing: &[String], base: &str) -> String {
  if !existing.iter().any(|name| name == base) {
    return base.to_string();
  }
  let mut n = 2;
  loop {
    let candidate = format!("{base} {n}");
    if !existing.iter().any(|name| name == &candidate) {
      return candidate;
    }
    n += 1;
  }
}

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

/// Every file under `root`, relative and slash-separated, sorted case-insensitively.
/// Used when the open folder has no Git repository, so `git ls-files` cannot list it.
/// Symlinked directories are not followed, and the walk is capped so a stray
/// `node_modules` cannot stall the explorer.
pub fn walk_folder_paths(root: &Path) -> Vec<String> {
  const MAX_ENTRIES: usize = 50_000;
  let mut paths = Vec::new();
  let mut pending = std::collections::VecDeque::from([String::new()]);
  while let Some(relative) = pending.pop_front() {
    if paths.len() >= MAX_ENTRIES {
      break;
    }
    let dir = if relative.is_empty() {
      root.to_path_buf()
    } else {
      root.join(&relative)
    };
    let Ok(read) = fs::read_dir(&dir) else {
      continue;
    };
    for child in read.flatten() {
      let name = child.file_name();
      let name = name.to_string_lossy();
      let path = if relative.is_empty() {
        name.to_string()
      } else {
        format!("{relative}/{name}")
      };
      if is_hard_hidden(&path) {
        continue;
      }
      let Ok(file_type) = child.file_type() else {
        continue;
      };
      if file_type.is_dir() {
        pending.push_back(path);
      } else {
        paths.push(path);
      }
    }
  }
  paths.sort_by_cached_key(|path| path.to_lowercase());
  paths
}

async fn collect_repository_entries(root: &Path) -> Result<Vec<ExplorerEntry>> {
  if GitRepository::open_optional(root)?.is_none() {
    return Ok(
      walk_folder_paths(root)
        .into_iter()
        .map(|path| explorer_entry(root, path, false, false))
        .collect(),
    );
  }
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

impl Core {
  pub async fn list_repository_tree(&self, id: SessionId) -> Result<Vec<ExplorerEntry>> {
    let root = self.repo_root(id)?;
    collect_repository_entries(&root).await
  }

  pub fn list_repository_children(&self, id: SessionId, path: &str) -> Result<Vec<ExplorerEntry>> {
    let root = self.repo_root(id)?;
    collect_directory_entries(&root, path)
  }

  pub fn read_file_content(&self, id: SessionId, path: &str) -> Result<FileContent> {
    let root = self.repo_root(id)?;
    let target = resolve_repo_file(&root, path)?;
    let metadata = fs::metadata(&target)?;
    let len = metadata.len();

    match read_plan(path, len) {
      ReadPlan::Oversized => Ok(oversized(path)),
      // A PDF's bytes stay on disk; the app asks for rendered pages instead.
      ReadPlan::Pdf => Ok(FileContent {
        content_hash: crate::content_hash::stamp(len, modified_nanos(&metadata)),
        path: path.to_string(),
        content: String::new(),
        bytes: None,
        language: None,
        file_type: "pdf".to_string(),
      }),
      ReadPlan::Image => {
        let bytes = fs::read(&target)?;
        Ok(FileContent {
          content_hash: crate::content_hash::sha256_bytes(&bytes),
          path: path.to_string(),
          content: String::new(),
          bytes: Some(Arc::from(bytes)),
          language: None,
          file_type: "image".to_string(),
        })
      }
      ReadPlan::Text => {
        let bytes = fs::read(&target)?;
        // Binary detection: check for null bytes in the first 8 KB.
        let check_len = bytes.len().min(BINARY_CHECK_SIZE);
        if bytes[..check_len].contains(&0) {
          return Ok(binary(path));
        }
        match String::from_utf8(bytes) {
          Ok(content) => Ok(FileContent {
            content_hash: crate::content_hash::sha256_utf8(&content),
            path: path.to_string(),
            content,
            bytes: None,
            language: detect_language(path),
            file_type: "text".to_string(),
          }),
          Err(_) => Ok(binary(path)),
        }
      }
    }
  }

  /// Rasterize one page of a PDF in the repository. Synchronous: callers run it off the UI thread.
  pub fn render_pdf_page(&self, id: SessionId, path: &str, page: usize, max_edge: u32) -> Result<PdfPageImage> {
    let root = self.repo_root(id)?;
    let target = resolve_repo_file(&root, path)?;
    // The viewer already refused to open a file this big, but the page request arrives on its own.
    if fs::metadata(&target)?.len() > MAX_PDF_SIZE {
      return Err(Error::Other("This PDF is too large to display".into()));
    }
    crate::pdf::render_page(&target, page, max_edge)
  }

  pub fn fuzzy_find_files(&self, id: SessionId, query: &str, max_results: usize) -> Result<Vec<FuzzyFileResult>> {
    let runtime = self.runtimes.runtime_for_session(id).ok_or(Error::NoRepository)?;
    runtime.fuzzy_find(query, max_results)
  }

  pub async fn search_file_contents(
    &self,
    id: SessionId,
    query: &str,
    max_results: usize,
  ) -> Result<Vec<ContentSearchResult>> {
    if query.is_empty() {
      return Ok(vec![]);
    }

    let root = self.repo_root(id)?;

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
        query,
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

  #[test]
  fn next_entry_name_numbers_taken_names() {
    let existing = vec!["New File".to_string(), "New File 2".to_string(), "other".to_string()];
    assert_eq!(next_entry_name(&existing, "New File"), "New File 3");
    assert_eq!(next_entry_name(&[], "New Folder"), "New Folder");
    assert_eq!(
      next_entry_name(&["New Folder 2".to_string()], "New Folder"),
      "New Folder"
    );
  }

  #[test]
  fn images_and_pdfs_get_their_own_budget() {
    // The text budget used to run first, so every image over 5 MiB came back as "large".
    assert_eq!(read_plan("shot.png", 30 * 1024 * 1024), ReadPlan::Image);
    assert_eq!(read_plan("manual.pdf", 30 * 1024 * 1024), ReadPlan::Pdf);
    assert_eq!(read_plan("notes.txt", 30 * 1024 * 1024), ReadPlan::Oversized);

    assert_eq!(read_plan("shot.png", MAX_IMAGE_SIZE + 1), ReadPlan::Oversized);
    assert_eq!(read_plan("manual.pdf", MAX_PDF_SIZE + 1), ReadPlan::Oversized);
    assert_eq!(read_plan("notes.txt", MAX_FILE_SIZE), ReadPlan::Text);
  }

  #[test]
  fn read_plan_is_case_insensitive_about_extensions() {
    assert_eq!(read_plan("SHOT.PNG", 1), ReadPlan::Image);
    assert_eq!(read_plan("MANUAL.PDF", 1), ReadPlan::Pdf);
    assert_eq!(read_plan("photo.avif", 1), ReadPlan::Text);
  }
}
