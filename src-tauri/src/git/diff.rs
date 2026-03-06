use std::fs;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::STANDARD;

use crate::error::Result;
use crate::git::repository::GitRepository;
use crate::types::DiffContent;

const IMAGE_EXTENSIONS: &[&str] = &[
  "png", "jpg", "jpeg", "gif", "bmp", "webp", "ico", "avif", "tiff", "svg",
];

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

fn read_head_blob_base64(repo: &git2::Repository, path: &str) -> Option<String> {
  let head = repo.head().ok()?;
  let tree = head.peel_to_tree().ok()?;
  let entry = tree.get_path(Path::new(path)).ok()?;
  let blob = repo.find_blob(entry.id()).ok()?;
  Some(blob_to_data_uri(blob.content(), path))
}

fn read_index_blob_base64(repo: &git2::Repository, path: &str) -> Option<String> {
  let mut index = repo.index().ok()?;
  index.read(true).ok()?;
  let entry = index.get_path(Path::new(path), 0)?;
  let blob = repo.find_blob(entry.id).ok()?;
  Some(blob_to_data_uri(blob.content(), path))
}

fn read_working_tree_file_base64(abs_path: &Path, rel_path: &str) -> Option<String> {
  let bytes = fs::read(abs_path).ok()?;
  Some(blob_to_data_uri(&bytes, rel_path))
}

pub fn get_file_diff(repo: &GitRepository, path: &str, staged: bool) -> Result<DiffContent> {
  let repo_root = repo.root();
  let abs_path = repo_root.join(path);
  let r = repo.inner();

  if is_image_file(path) {
    let original = if staged {
      read_head_blob_base64(r, path)
    } else {
      read_index_blob_base64(r, path)
    };

    let modified = if staged {
      read_index_blob_base64(r, path)
    } else {
      read_working_tree_file_base64(&abs_path, path)
    };

    return Ok(DiffContent {
      path: path.to_string(),
      original: original.unwrap_or_default(),
      modified: modified.unwrap_or_default(),
      original_language: None,
      file_type: "image".to_string(),
    });
  }

  let original = if staged {
    read_head_blob(r, path)
  } else {
    read_index_blob(r, path)
  };

  let modified = if staged {
    read_index_blob(r, path)
  } else {
    read_working_tree_file(&abs_path)
  };

  let language = detect_language(path);

  Ok(DiffContent {
    path: path.to_string(),
    original: original.unwrap_or_default(),
    modified: modified.unwrap_or_default(),
    original_language: language,
    file_type: "text".to_string(),
  })
}

fn read_head_blob(repo: &git2::Repository, path: &str) -> Option<String> {
  let head = repo.head().ok()?;
  let tree = head.peel_to_tree().ok()?;
  let entry = tree.get_path(Path::new(path)).ok()?;
  let blob = repo.find_blob(entry.id()).ok()?;
  String::from_utf8(blob.content().to_vec()).ok()
}

fn read_index_blob(repo: &git2::Repository, path: &str) -> Option<String> {
  let mut index = repo.index().ok()?;
  index.read(true).ok()?;
  let entry = index.get_path(Path::new(path), 0)?;
  let blob = repo.find_blob(entry.id).ok()?;
  String::from_utf8(blob.content().to_vec()).ok()
}

fn read_working_tree_file(abs_path: &Path) -> Option<String> {
  fs::read_to_string(abs_path).ok()
}

pub fn detect_language(path: &str) -> Option<String> {
  let ext = Path::new(path).extension()?.to_str()?;
  let lang = match ext {
    "rs" => "rust",
    "ts" | "tsx" => "typescript",
    "js" | "jsx" => "javascript",
    "json" => "json",
    "html" => "html",
    "css" => "css",
    "scss" => "scss",
    "md" => "markdown",
    "toml" => "toml",
    "yaml" | "yml" => "yaml",
    "py" => "python",
    "go" => "go",
    "sh" | "bash" | "zsh" => "shell",
    "sql" => "sql",
    "xml" => "xml",
    "svg" => "xml",
    _ => return None,
  };
  Some(lang.to_string())
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
  fn detect_language_no_extension() {
    assert_eq!(detect_language("Makefile"), None);
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
}
