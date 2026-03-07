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
}
