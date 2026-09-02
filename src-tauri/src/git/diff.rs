use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::Result;
use crate::git::repository::GitRepository;
use crate::types::{DiffContent, DiffHunk, DiffLine};

const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "avif", "tiff", "svg"];

pub fn is_image_file(path: &str) -> bool {
  Path::new(path)
    .extension()
    .and_then(|e| e.to_str())
    .is_some_and(|ext| IMAGE_EXTENSIONS.contains(&ext.to_lowercase().as_str()))
}

fn image_mime_type(ext: &str) -> &str {
  match ext.to_lowercase().as_str() {
    "png" => "image/png",
    "jpg" | "jpeg" => "image/jpeg",
    "gif" => "image/gif",
    "bmp" => "image/bmp",
    "webp" => "image/webp",
    "ico" => "image/x-icon",
    "avif" => "image/avif",
    "tiff" => "image/tiff",
    "svg" => "image/svg+xml",
    _ => "application/octet-stream",
  }
}

pub fn blob_to_data_uri(blob: &[u8], path: &str) -> String {
  let ext = Path::new(path).extension().and_then(|e| e.to_str()).unwrap_or("");
  let mime = image_mime_type(ext);
  let encoded = STANDARD.encode(blob);
  format!("data:{};base64,{}", mime, encoded)
}

pub struct ScmFileDiff {
  pub content: DiffContent,
  pub hunks: Vec<DiffHunk>,
}

pub fn scm_file_diff(repo: &GitRepository, path: &str, staged: bool) -> Result<ScmFileDiff> {
  let workdir = repo.root();
  let inner = repo.inner();
  let mut opts = git2::DiffOptions::new();
  opts.pathspec(path).context_lines(3);
  if !staged {
    opts
      .include_untracked(true)
      .recurse_untracked_dirs(true)
      .show_untracked_content(true);
  }

  let diff = if staged {
    let head_tree = inner.head().ok().and_then(|head| head.peel_to_tree().ok());
    inner.diff_tree_to_index(head_tree.as_ref(), None, Some(&mut opts))?
  } else {
    inner.diff_index_to_workdir(None, Some(&mut opts))?
  };

  let mut old_file: Option<(git2::Oid, Option<std::path::PathBuf>, bool, bool)> = None;
  let mut new_file: Option<(git2::Oid, Option<std::path::PathBuf>, bool, bool)> = None;
  let mut content_path = path.to_string();
  diff.foreach(
    &mut |delta, _| {
      if let Some(new_path) = delta.new_file().path() {
        content_path = new_path.to_string_lossy().replace('\\', "/");
      } else if let Some(old_path) = delta.old_file().path() {
        content_path = old_path.to_string_lossy().replace('\\', "/");
      }
      old_file = Some((
        delta.old_file().id(),
        delta.old_file().path().map(Path::to_path_buf),
        delta.old_file().exists(),
        delta.old_file().is_binary(),
      ));
      new_file = Some((
        delta.new_file().id(),
        delta.new_file().path().map(Path::to_path_buf),
        delta.new_file().exists(),
        delta.new_file().is_binary(),
      ));
      true
    },
    None,
    None,
    None,
  )?;

  let original_bytes = old_file
    .as_ref()
    .and_then(|(id, file_path, exists, _)| read_diff_side(inner, workdir, *id, file_path.as_deref(), *exists));
  let modified_bytes = new_file
    .as_ref()
    .and_then(|(id, file_path, exists, _)| read_diff_side(inner, workdir, *id, file_path.as_deref(), *exists));
  let binary_flag =
    old_file.is_some_and(|(_, _, _, binary)| binary) || new_file.is_some_and(|(_, _, _, binary)| binary);

  if is_image_file(path) || is_image_file(&content_path) {
    return Ok(ScmFileDiff {
      content: DiffContent {
        path: content_path,
        original: original_bytes
          .as_deref()
          .map(|bytes| blob_to_data_uri(bytes, path))
          .unwrap_or_default(),
        modified: modified_bytes
          .as_deref()
          .map(|bytes| blob_to_data_uri(bytes, path))
          .unwrap_or_default(),
        original_language: None,
        file_type: "image".to_string(),
      },
      hunks: Vec::new(),
    });
  }

  if binary_flag || is_binary_bytes(original_bytes.as_deref()) || is_binary_bytes(modified_bytes.as_deref()) {
    return Ok(ScmFileDiff {
      content: DiffContent {
        path: content_path,
        original: String::new(),
        modified: String::new(),
        original_language: None,
        file_type: "binary".to_string(),
      },
      hunks: Vec::new(),
    });
  }

  let original = original_bytes
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .unwrap_or_default();
  let modified = modified_bytes
    .and_then(|bytes| String::from_utf8(bytes).ok())
    .unwrap_or_default();
  let mut hunks = collect_diff_hunks(&diff)?;
  if hunks.is_empty() && original.is_empty() && !modified.is_empty() {
    hunks = all_add_hunks(&modified);
  }

  Ok(ScmFileDiff {
    content: DiffContent {
      path: content_path,
      original,
      modified,
      original_language: detect_language(path),
      file_type: "text".to_string(),
    },
    hunks,
  })
}

struct HunkWalk {
  hunks: Vec<DiffHunk>,
  current: Option<DiffHunk>,
  old_line: usize,
  new_line: usize,
}

fn collect_diff_hunks(diff: &git2::Diff<'_>) -> Result<Vec<DiffHunk>> {
  let walk = std::cell::RefCell::new(HunkWalk {
    hunks: Vec::new(),
    current: None,
    old_line: 0,
    new_line: 0,
  });
  diff.foreach(
    &mut |_, _| true,
    None,
    Some(&mut |_, hunk| {
      let mut walk = walk.borrow_mut();
      if let Some(done) = walk.current.take() {
        walk.hunks.push(done);
      }
      walk.old_line = hunk.old_start() as usize;
      walk.new_line = hunk.new_start() as usize;
      walk.current = Some(DiffHunk {
        header: format_hunk_header(&hunk),
        old_start: hunk.old_start() as usize,
        old_lines: hunk.old_lines() as usize,
        new_start: hunk.new_start() as usize,
        new_lines: hunk.new_lines() as usize,
        lines: Vec::new(),
      });
      true
    }),
    Some(&mut |_, _, line| {
      let mut walk = walk.borrow_mut();
      let content = diff_line_content(&line);
      let old_line = walk.old_line;
      let new_line = walk.new_line;
      let Some(hunk) = walk.current.as_mut() else {
        return true;
      };
      match line.origin() {
        '+' => {
          hunk.lines.push(DiffLine {
            content,
            line_type: "add".into(),
            old_line_number: None,
            new_line_number: Some(new_line),
          });
          walk.new_line = new_line + 1;
        }
        '-' => {
          hunk.lines.push(DiffLine {
            content,
            line_type: "remove".into(),
            old_line_number: Some(old_line),
            new_line_number: None,
          });
          walk.old_line = old_line + 1;
        }
        ' ' => {
          hunk.lines.push(DiffLine {
            content,
            line_type: "context".into(),
            old_line_number: Some(old_line),
            new_line_number: Some(new_line),
          });
          walk.old_line = old_line + 1;
          walk.new_line = new_line + 1;
        }
        _ => {}
      }
      true
    }),
  )?;
  let mut walk = walk.into_inner();
  if let Some(done) = walk.current.take() {
    walk.hunks.push(done);
  }
  Ok(walk.hunks)
}

fn format_hunk_header(hunk: &git2::DiffHunk<'_>) -> String {
  let raw = std::str::from_utf8(hunk.header()).unwrap_or("").trim_end();
  let header_text = raw.find(" @@").and_then(|end| {
    let rest = raw.get(end + 3..)?.trim();
    if rest.is_empty() { None } else { Some(rest) }
  });
  let mut header = format!(
    "@@ -{} +{} @@",
    format_diff_range(hunk.old_start(), hunk.old_lines()),
    format_diff_range(hunk.new_start(), hunk.new_lines()),
  );
  if let Some(text) = header_text {
    header.push(' ');
    header.push_str(text);
  }
  header
}

fn format_diff_range(start: u32, lines: u32) -> String {
  if lines == 1 {
    format!("{start}")
  } else {
    format!("{start},{lines}")
  }
}

fn diff_line_content(line: &git2::DiffLine<'_>) -> String {
  let text = std::str::from_utf8(line.content()).unwrap_or("");
  text.trim_end_matches('\n').trim_end_matches('\r').to_string()
}

fn all_add_hunks(modified: &str) -> Vec<DiffHunk> {
  let lines: Vec<&str> = modified.lines().collect();
  if lines.is_empty() {
    return Vec::new();
  }
  let new_lines = lines.len();
  vec![DiffHunk {
    header: format!("@@ -0,0 +{} @@", format_diff_range(1, new_lines as u32)),
    old_start: 0,
    old_lines: 0,
    new_start: 1,
    new_lines,
    lines: lines
      .into_iter()
      .enumerate()
      .map(|(index, content)| DiffLine {
        content: content.to_string(),
        line_type: "add".into(),
        old_line_number: None,
        new_line_number: Some(index + 1),
      })
      .collect(),
  }]
}

fn read_diff_side(
  repo: &git2::Repository,
  workdir: &Path,
  id: git2::Oid,
  path: Option<&Path>,
  exists: bool,
) -> Option<Vec<u8>> {
  if !exists {
    return None;
  }
  if !id.is_zero()
    && let Ok(blob) = repo.find_blob(id)
  {
    return Some(blob.content().to_vec());
  }
  fs::read(workdir.join(path?)).ok()
}

fn is_binary_bytes(bytes: Option<&[u8]>) -> bool {
  bytes.is_some_and(|bytes| bytes.contains(&0))
}

fn detect_language_by_filename(path: &str) -> Option<&'static str> {
  let filename = Path::new(path).file_name()?.to_str()?;

  // Exact filename matches
  let lang = match filename {
    "justfile" | "Justfile" | ".justfile" => "justfile",
    "Makefile" | "GNUmakefile" => "shell",
    "Gemfile" | "Rakefile" => "ruby",
    ".gitignore" | ".gitattributes" | ".editorconfig" | ".gitconfig" => "ini",
    "Cargo.lock" => "toml",
    _ => {
      // Prefix-based matches
      if filename == "Dockerfile" || filename.starts_with("Dockerfile.") {
        "dockerfile"
      } else if filename == ".env" || filename.starts_with(".env.") {
        "dotenv"
      } else {
        return None;
      }
    }
  };
  Some(lang)
}

fn detect_language_by_extension(ext: &str) -> Option<&'static str> {
  let lang = match ext {
    "rs" => "rust",
    "ts" | "tsx" => "typescript",
    "js" | "jsx" | "mjs" | "cjs" => "javascript",
    "json" | "jsonc" => "json",
    "html" | "htm" => "html",
    "css" => "css",
    "scss" => "scss",
    "less" => "less",
    "md" => "markdown",
    "mdx" => "mdx",
    "toml" => "toml",
    "yaml" | "yml" => "yaml",
    "py" | "pyw" => "python",
    "go" => "go",
    "sh" | "bash" | "zsh" => "shell",
    "sql" => "sql",
    "xml" | "xsl" | "xsd" => "xml",
    "svg" => "xml",
    "java" => "java",
    "kt" | "kts" => "kotlin",
    "swift" => "swift",
    "dart" => "dart",
    "c" | "h" => "c",
    "cpp" | "cc" | "cxx" | "hpp" | "hh" | "hxx" => "cpp",
    "cs" => "csharp",
    "fs" | "fsi" | "fsx" => "fsharp",
    "rb" => "ruby",
    "php" => "php",
    "lua" => "lua",
    "pl" | "pm" => "perl",
    "r" => "r",
    "jl" => "julia",
    "scala" | "sc" | "sbt" => "scala",
    "clj" | "cljs" | "cljc" | "edn" => "clojure",
    "ex" | "exs" => "elixir",
    "coffee" => "coffeescript",
    "tf" | "tfvars" | "hcl" => "hcl",
    "graphql" | "gql" => "graphql",
    "proto" => "proto",
    "dockerfile" => "dockerfile",
    "ps1" | "psm1" | "psd1" => "powershell",
    "bat" | "cmd" => "bat",
    "ini" | "properties" | "cfg" => "ini",
    "m" => "objective-c",
    "pas" => "pascal",
    "scm" | "ss" | "rkt" => "scheme",
    "tcl" => "tcl",
    "hbs" | "handlebars" => "handlebars",
    "pug" | "jade" => "pug",
    "rst" => "restructuredtext",
    "sol" => "sol",
    "wgsl" => "wgsl",
    "bicep" => "bicep",
    "liquid" => "liquid",
    "env" => "dotenv",
    _ => return None,
  };
  Some(lang)
}

pub fn detect_language(path: &str) -> Option<String> {
  if let Some(lang) = detect_language_by_filename(path) {
    return Some(lang.to_string());
  }

  let ext = Path::new(path).extension()?.to_str()?;
  detect_language_by_extension(ext).map(|l| l.to_string())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn detect_language_rs() {
    assert_eq!(detect_language("main.rs"), Some("rust".to_string()));
  }

  #[test]
  fn detect_language_ts() {
    assert_eq!(detect_language("app.ts"), Some("typescript".to_string()));
  }

  #[test]
  fn detect_language_tsx() {
    assert_eq!(detect_language("component.tsx"), Some("typescript".to_string()));
  }

  #[test]
  fn detect_language_js() {
    assert_eq!(detect_language("index.js"), Some("javascript".to_string()));
  }

  #[test]
  fn detect_language_json() {
    assert_eq!(detect_language("package.json"), Some("json".to_string()));
  }

  #[test]
  fn detect_language_html() {
    assert_eq!(detect_language("index.html"), Some("html".to_string()));
  }

  #[test]
  fn detect_language_css() {
    assert_eq!(detect_language("style.css"), Some("css".to_string()));
  }

  #[test]
  fn detect_language_md() {
    assert_eq!(detect_language("README.md"), Some("markdown".to_string()));
  }

  #[test]
  fn detect_language_py() {
    assert_eq!(detect_language("script.py"), Some("python".to_string()));
  }

  #[test]
  fn detect_language_go() {
    assert_eq!(detect_language("main.go"), Some("go".to_string()));
  }

  #[test]
  fn detect_language_sh() {
    assert_eq!(detect_language("build.sh"), Some("shell".to_string()));
  }

  #[test]
  fn detect_language_unknown_ext() {
    assert_eq!(detect_language("file.xyz"), None);
  }

  #[test]
  fn detect_language_makefile() {
    assert_eq!(detect_language("Makefile"), Some("shell".to_string()));
  }

  #[test]
  fn detect_language_dockerfile() {
    assert_eq!(detect_language("Dockerfile"), Some("dockerfile".to_string()));
    assert_eq!(detect_language("Dockerfile.prod"), Some("dockerfile".to_string()));
  }

  #[test]
  fn detect_language_justfile() {
    assert_eq!(detect_language("justfile"), Some("justfile".to_string()));
    assert_eq!(detect_language("Justfile"), Some("justfile".to_string()));
  }

  #[test]
  fn detect_language_dotenv() {
    assert_eq!(detect_language(".env"), Some("dotenv".to_string()));
    assert_eq!(detect_language(".env.local"), Some("dotenv".to_string()));
  }

  #[test]
  fn detect_language_gitignore() {
    assert_eq!(detect_language(".gitignore"), Some("ini".to_string()));
  }

  #[test]
  fn detect_language_java() {
    assert_eq!(detect_language("Main.java"), Some("java".to_string()));
  }

  #[test]
  fn detect_language_kotlin() {
    assert_eq!(detect_language("App.kt"), Some("kotlin".to_string()));
  }

  #[test]
  fn detect_language_c() {
    assert_eq!(detect_language("main.c"), Some("c".to_string()));
    assert_eq!(detect_language("lib.h"), Some("c".to_string()));
  }

  #[test]
  fn detect_language_cpp() {
    assert_eq!(detect_language("main.cpp"), Some("cpp".to_string()));
    assert_eq!(detect_language("lib.hpp"), Some("cpp".to_string()));
  }

  #[test]
  fn detect_language_ruby() {
    assert_eq!(detect_language("app.rb"), Some("ruby".to_string()));
    assert_eq!(detect_language("Gemfile"), Some("ruby".to_string()));
  }

  #[test]
  fn detect_language_php() {
    assert_eq!(detect_language("index.php"), Some("php".to_string()));
  }

  #[test]
  fn detect_language_lua() {
    assert_eq!(detect_language("init.lua"), Some("lua".to_string()));
  }

  #[test]
  fn detect_language_elixir() {
    assert_eq!(detect_language("app.ex"), Some("elixir".to_string()));
    assert_eq!(detect_language("test.exs"), Some("elixir".to_string()));
  }

  #[test]
  fn detect_language_no_extension_unknown() {
    assert_eq!(detect_language("README"), None);
  }

  #[test]
  fn is_image_file_png() {
    assert!(is_image_file("photo.png"));
  }

  #[test]
  fn is_image_file_unknown_ext() {
    assert!(!is_image_file("file.txt"));
  }

  #[test]
  fn is_image_file_case_insensitive() {
    assert!(is_image_file("FILE.PNG"));
  }

  #[test]
  fn is_image_file_no_extension() {
    assert!(!is_image_file("noext"));
  }

  #[test]
  fn blob_to_data_uri_basic() {
    let blob = b"hello";
    let uri = blob_to_data_uri(blob, "test.png");
    assert!(uri.starts_with("data:image/png;base64,"));
    assert!(uri.len() > "data:image/png;base64,".len());
  }

  #[test]
  fn blob_to_data_uri_empty() {
    let uri = blob_to_data_uri(&[], "empty.jpg");
    assert_eq!(uri, "data:image/jpeg;base64,");
  }

  fn init_repo() -> (tempfile::TempDir, GitRepository) {
    let directory = tempfile::TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    let root = repo.workdir().unwrap();
    std::fs::write(root.join("README.md"), "hello\n").unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new("README.md")).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "initial\n", &tree, &[]).unwrap();
    let git_repo = GitRepository::open(directory.path()).unwrap();
    (directory, git_repo)
  }

  fn git_cli_diff(root: &Path, path: &str, staged: bool) -> String {
    let mut command = std::process::Command::new("git");
    command.arg("diff");
    if staged {
      command.arg("--cached");
    }
    command.args(["--", path]).current_dir(root);
    let output = command.output().unwrap();
    String::from_utf8_lossy(&output.stdout).into_owned()
  }

  #[test]
  fn git2_hunk_ids_match_parse_unified_diff() {
    let (directory, repo) = init_repo();
    std::fs::write(directory.path().join("README.md"), "hello world\n").unwrap();
    let diff = scm_file_diff(&repo, "README.md", false).unwrap();
    let cli = git_cli_diff(directory.path(), "README.md", false);
    let parsed = crate::git::hunk::parse_unified_diff(&cli);
    let git2_ids: Vec<String> = diff.hunks.iter().map(crate::git::hunk::hunk_id).collect();
    let parsed_ids: Vec<String> = parsed.iter().map(crate::git::hunk::hunk_id).collect();
    assert_eq!(git2_ids, parsed_ids);
    assert!(!git2_ids.is_empty());
    assert_eq!(diff.content.original, "hello\n");
    assert_eq!(diff.content.modified, "hello world\n");
  }

  #[test]
  fn untracked_is_all_add_hunk() {
    let (directory, repo) = init_repo();
    std::fs::write(directory.path().join("new.txt"), "fresh\n").unwrap();
    let diff = scm_file_diff(&repo, "new.txt", false).unwrap();
    assert_eq!(diff.content.original, "");
    assert_eq!(diff.content.modified, "fresh\n");
    assert!(!diff.hunks.is_empty());
    assert!(
      diff
        .hunks
        .iter()
        .all(|hunk| hunk.lines.iter().all(|line| line.line_type != "remove"))
    );
    assert!(
      diff
        .hunks
        .iter()
        .any(|hunk| hunk.lines.iter().any(|line| line.line_type == "add"))
    );
  }

  #[test]
  fn deleted_has_no_new_side() {
    let (directory, repo) = init_repo();
    std::fs::remove_file(directory.path().join("README.md")).unwrap();
    let diff = scm_file_diff(&repo, "README.md", false).unwrap();
    assert_eq!(diff.content.original, "hello\n");
    assert_eq!(diff.content.modified, "");
    assert!(
      diff
        .hunks
        .iter()
        .all(|hunk| hunk.lines.iter().all(|line| line.line_type != "add"))
    );
  }

  #[test]
  fn no_newline_marker_is_not_a_diff_line() {
    let (directory, repo) = init_repo();
    std::fs::write(directory.path().join("README.md"), "hello").unwrap();
    let diff = scm_file_diff(&repo, "README.md", false).unwrap();
    assert!(diff.hunks.iter().flat_map(|hunk| hunk.lines.iter()).all(
      |line| !line.content.contains("No newline") && matches!(line.line_type.as_str(), "add" | "remove" | "context")
    ));
    assert!(!diff.hunks.is_empty());
  }

  #[test]
  fn image_has_no_hunks() {
    let (directory, repo) = init_repo();
    std::fs::write(directory.path().join("photo.png"), b"\x89PNG\r\n").unwrap();
    let diff = scm_file_diff(&repo, "photo.png", false).unwrap();
    assert_eq!(diff.content.file_type, "image");
    assert!(diff.hunks.is_empty());
    assert!(diff.content.modified.starts_with("data:image/png;base64,"));
  }

  #[test]
  fn binary_has_no_hunks() {
    let (directory, repo) = init_repo();
    std::fs::write(directory.path().join("data.bin"), b"hello\0world").unwrap();
    let diff = scm_file_diff(&repo, "data.bin", false).unwrap();
    assert_eq!(diff.content.file_type, "binary");
    assert!(diff.hunks.is_empty());
  }
}
