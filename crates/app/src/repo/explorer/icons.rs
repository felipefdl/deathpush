use deathpush_core::config::settings::{TreeDensity, TreeIcons};
use deathpush_core::theme::ThemeKind;
use gpui_kit::*;

use super::material;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconKind {
  /// No icons at all.
  None,
  /// Monochrome Lucide icons by category, tinted with the row color.
  Lucide,
  /// Full-color Material Icon Theme icons, per file name and folder name.
  Material { light: bool },
}

impl IconKind {
  pub fn new(icons: TreeIcons, theme: ThemeKind) -> Self {
    match icons {
      TreeIcons::Minimal => Self::None,
      TreeIcons::Standard => Self::Lucide,
      TreeIcons::Complete => Self::Material {
        light: theme == ThemeKind::Light,
      },
    }
  }
}

/// Asset path for a row.
pub fn icon_for(kind: IconKind, name: &str, is_directory: bool, expanded: bool) -> Option<&'static str> {
  let name = file_name(name);
  match kind {
    IconKind::None => None,
    IconKind::Lucide => Some(lucide_icon(name, is_directory, expanded)),
    IconKind::Material { light } => Some(if is_directory {
      material::folder(name, expanded, light)
    } else {
      material::file(name, light)
    }),
  }
}

/// Material icons carry their own colors; Lucide icons are masks tinted by `color`.
pub fn render_icon(kind: IconKind, path: &'static str, color: Hsla) -> AnyElement {
  if matches!(kind, IconKind::Material { .. }) {
    img(path).size(px(16.0)).flex_shrink_0().into_any_element()
  } else {
    svg()
      .path(path)
      .size(px(16.0))
      .flex_shrink_0()
      .text_color(color)
      .into_any_element()
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

fn lucide_icon(name: &str, is_directory: bool, expanded: bool) -> &'static str {
  if is_directory {
    return if expanded {
      "icons/folder-open.svg"
    } else {
      "icons/folder.svg"
    };
  }
  match extension(name).map(|ext| ext.to_ascii_lowercase()).as_deref() {
    Some("png" | "jpg" | "jpeg" | "gif" | "webp" | "ico" | "bmp" | "svg") => "icons/file-image.svg",
    Some("json" | "jsonc") => "icons/file-braces.svg",
    Some("md" | "mdx" | "txt") => "icons/file-text.svg",
    Some("pdf" | "zip" | "tar" | "gz") => "icons/file-archive.svg",
    Some(
      "rs" | "ts" | "tsx" | "js" | "mjs" | "cjs" | "jsx" | "py" | "go" | "java" | "kt" | "kts" | "swift" | "c" | "h"
      | "cpp" | "cc" | "hpp" | "cs" | "rb" | "php" | "lua" | "zig" | "sh" | "bash" | "zsh" | "html" | "htm" | "css"
      | "scss" | "less" | "toml" | "yaml" | "yml" | "sql",
    ) => "icons/file-code.svg",
    _ => "icons/file.svg",
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  const MATERIAL: IconKind = IconKind::Material { light: false };

  #[test]
  fn material_maps_extensions_and_whole_names() {
    assert_eq!(
      icon_for(MATERIAL, "main.rs", false, false),
      Some("material-icons/rust.svg")
    );
    assert_eq!(
      icon_for(MATERIAL, "src/Cargo.lock", false, false),
      Some("material-icons/lock.svg")
    );
    assert_eq!(
      icon_for(MATERIAL, "app.test.ts", false, false),
      Some("material-icons/test-ts.svg")
    );
  }

  #[test]
  fn material_falls_back_to_the_default_file() {
    assert_eq!(
      icon_for(MATERIAL, "notes.unknown", false, false),
      Some("material-icons/file.svg")
    );
  }

  #[test]
  fn material_uses_named_folder_icons() {
    assert_eq!(
      icon_for(MATERIAL, "src", true, false),
      Some("material-icons/folder-src.svg")
    );
    assert_eq!(
      icon_for(MATERIAL, "src", true, true),
      Some("material-icons/folder-src-open.svg")
    );
    assert_eq!(
      icon_for(MATERIAL, "whatever", true, false),
      Some("material-icons/folder.svg")
    );
  }

  #[test]
  fn material_picks_the_light_variant_when_the_theme_is_light() {
    let light = IconKind::Material { light: true };
    assert_eq!(
      icon_for(light, "bun.lockb", false, false),
      Some("material-icons/bun_light.svg")
    );
    assert_eq!(
      icon_for(MATERIAL, "bun.lockb", false, false),
      Some("material-icons/bun.svg")
    );
  }

  #[test]
  fn lucide_maps_by_category() {
    assert_eq!(
      icon_for(IconKind::Lucide, "shot.png", false, false),
      Some("icons/file-image.svg")
    );
    assert_eq!(
      icon_for(IconKind::Lucide, "main.rs", false, false),
      Some("icons/file-code.svg")
    );
    assert_eq!(
      icon_for(IconKind::Lucide, "src", true, true),
      Some("icons/folder-open.svg")
    );
  }

  #[test]
  fn minimal_has_no_icons() {
    assert_eq!(icon_for(IconKind::None, "main.rs", false, false), None);
    assert_eq!(icon_for(IconKind::None, "src", true, true), None);
  }

  #[test]
  fn kind_follows_the_setting_and_the_theme() {
    assert_eq!(IconKind::new(TreeIcons::Minimal, ThemeKind::Dark), IconKind::None);
    assert_eq!(IconKind::new(TreeIcons::Standard, ThemeKind::Light), IconKind::Lucide);
    assert_eq!(
      IconKind::new(TreeIcons::Complete, ThemeKind::Light),
      IconKind::Material { light: true }
    );
  }

  #[test]
  fn row_height_per_density() {
    assert_eq!(row_height(TreeDensity::Compact), 22.0);
    assert_eq!(row_height(TreeDensity::Default), 26.0);
    assert_eq!(row_height(TreeDensity::Relaxed), 30.0);
  }
}
