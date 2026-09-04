use deathpush_core::config::settings::{TreeDensity, TreeIcons};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
  None,
  Standard,
  Complete,
}

impl From<TreeIcons> for IconKind {
  fn from(value: TreeIcons) -> Self {
    match value {
      TreeIcons::Minimal => IconKind::None,
      TreeIcons::Standard => IconKind::Standard,
      TreeIcons::Complete => IconKind::Complete,
    }
  }
}

/// Asset path for a row. Standard: codicons by category. Complete: vscode-icons, falling back to Standard.
pub fn icon_for(kind: IconKind, name: &str, is_directory: bool, expanded: bool) -> Option<&'static str> {
  match kind {
    IconKind::None => None,
    IconKind::Standard => Some(standard_icon(name, is_directory, expanded)),
    IconKind::Complete => Some(complete_icon(name, is_directory, expanded)),
  }
}

pub fn row_height(density: TreeDensity) -> f32 {
  match density {
    TreeDensity::Compact => 22.0,
    TreeDensity::Default => 26.0,
    TreeDensity::Relaxed => 30.0,
  }
}

fn file_name(name: &str) -> &str {
  name.rsplit('/').next().unwrap_or(name)
}

fn extension(name: &str) -> Option<&str> {
  let base = file_name(name);
  match base.rsplit_once('.') {
    Some((stem, ext)) if !stem.is_empty() => Some(ext),
    _ => None,
  }
}

fn standard_icon(name: &str, is_directory: bool, expanded: bool) -> &'static str {
  if is_directory {
    return if expanded {
      "icons/folder-opened.svg"
    } else {
      "icons/folder.svg"
    };
  }
  match extension(name).map(|ext| ext.to_ascii_lowercase()).as_deref() {
    Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp") => "icons/file-media.svg",
    Some("json" | "jsonc") => "icons/json.svg",
    Some("md" | "mdx") => "icons/markdown.svg",
    Some("pdf" | "zip" | "tar" | "gz") => "icons/file-binary.svg",
    Some(
      "rs" | "ts" | "tsx" | "js" | "mjs" | "cjs" | "jsx" | "py" | "go" | "java" | "kt" | "kts" | "swift" | "c" | "h"
      | "cpp" | "cc" | "hpp" | "cs" | "rb" | "php" | "lua" | "zig" | "sh" | "bash" | "zsh" | "html" | "htm" | "css"
      | "scss" | "less" | "toml" | "yaml" | "yml" | "sql",
    ) => "icons/file-code.svg",
    _ => "icons/file.svg",
  }
}

fn complete_icon(name: &str, is_directory: bool, expanded: bool) -> &'static str {
  if is_directory {
    return if expanded {
      "file-icons/default_folder_opened.svg"
    } else {
      "file-icons/default_folder.svg"
    };
  }
  let base = file_name(name);
  let lower = base.to_ascii_lowercase();
  if lower == "cargo.toml" || lower == "cargo.lock" {
    return "file-icons/file_type_cargo.svg";
  }
  if lower == "package.json" {
    return "file-icons/file_type_npm.svg";
  }
  if lower.starts_with("dockerfile") {
    return "file-icons/file_type_docker.svg";
  }
  if lower == "makefile" {
    return "file-icons/file_type_gnu.svg";
  }
  if lower == "justfile" {
    return "file-icons/file_type_light_config.svg";
  }
  if lower.starts_with("license") {
    return "file-icons/file_type_license.svg";
  }
  if lower == ".gitignore" || lower == ".gitattributes" {
    return "file-icons/file_type_git.svg";
  }
  if lower == ".env" || lower.starts_with(".env.") {
    return "file-icons/file_type_dotenv.svg";
  }
  match extension(&lower) {
    Some("rs") => "file-icons/file_type_rust.svg",
    Some("ts") => "file-icons/file_type_typescript.svg",
    Some("tsx") => "file-icons/file_type_reactts.svg",
    Some("js" | "mjs" | "cjs") => "file-icons/file_type_js.svg",
    Some("jsx") => "file-icons/file_type_reactjs.svg",
    Some("json" | "jsonc") => "file-icons/file_type_json.svg",
    Some("md" | "mdx") => "file-icons/file_type_markdown.svg",
    Some("html" | "htm") => "file-icons/file_type_html.svg",
    Some("css") => "file-icons/file_type_css.svg",
    Some("scss") => "file-icons/file_type_scss.svg",
    Some("less") => "file-icons/file_type_less.svg",
    Some("toml") => "file-icons/file_type_toml.svg",
    Some("yaml" | "yml") => "file-icons/file_type_yaml.svg",
    Some("py") => "file-icons/file_type_python.svg",
    Some("go") => "file-icons/file_type_go.svg",
    Some("sh" | "bash" | "zsh") => "file-icons/file_type_shell.svg",
    Some("sql") => "file-icons/file_type_sql.svg",
    Some("java") => "file-icons/file_type_java.svg",
    Some("kt" | "kts") => "file-icons/file_type_kotlin.svg",
    Some("swift") => "file-icons/file_type_swift.svg",
    Some("c" | "h") => "file-icons/file_type_c.svg",
    Some("cpp" | "cc" | "hpp") => "file-icons/file_type_cpp.svg",
    Some("cs") => "file-icons/file_type_csharp.svg",
    Some("rb") => "file-icons/file_type_ruby.svg",
    Some("php") => "file-icons/file_type_php.svg",
    Some("lua") => "file-icons/file_type_lua.svg",
    Some("zig") => "file-icons/file_type_zig.svg",
    Some("xml") => "file-icons/file_type_xml.svg",
    Some("svg") => "file-icons/file_type_svg.svg",
    Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp") => "file-icons/file_type_image.svg",
    Some("lock") => "file-icons/default_file.svg",
    Some("txt") => "file-icons/file_type_text.svg",
    Some("pdf") => "file-icons/file_type_pdf.svg",
    Some("zip" | "tar" | "gz") => "file-icons/file_type_zip.svg",
    _ => "file-icons/default_file.svg",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn rust_maps_to_the_rust_icon() {
    assert_eq!(
      icon_for(IconKind::Complete, "main.rs", false, false),
      Some("file-icons/file_type_rust.svg")
    );
  }

  #[test]
  fn unknown_extension_falls_back_to_default_file() {
    assert_eq!(
      icon_for(IconKind::Complete, "notes.unknown", false, false),
      Some("file-icons/default_file.svg")
    );
  }

  #[test]
  fn standard_uses_codicon_categories() {
    assert_eq!(
      icon_for(IconKind::Standard, "shot.png", false, false),
      Some("icons/file-media.svg")
    );
    assert_eq!(
      icon_for(IconKind::Standard, "main.rs", false, false),
      Some("icons/file-code.svg")
    );
    assert_eq!(
      icon_for(IconKind::Standard, "src", true, true),
      Some("icons/folder-opened.svg")
    );
  }

  #[test]
  fn minimal_has_no_icons() {
    assert_eq!(icon_for(IconKind::None, "main.rs", false, false), None);
    assert_eq!(icon_for(IconKind::None, "src", true, true), None);
  }

  #[test]
  fn row_height_per_density() {
    assert_eq!(row_height(TreeDensity::Compact), 22.0);
    assert_eq!(row_height(TreeDensity::Default), 26.0);
    assert_eq!(row_height(TreeDensity::Relaxed), 30.0);
  }
}
