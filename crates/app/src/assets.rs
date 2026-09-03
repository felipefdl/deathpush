use std::borrow::Cow;

use gpui_kit::{AssetSource, Result, SharedString};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../../assets/"]
#[include = "icons/*.svg"]
#[include = "themes/*.json"]
#[include = "fonts/nerd-font/*.ttf"]
#[include = "brand/*"]
struct Embedded;

/// Our embedded assets first, gpui-kit's icon set second.
pub struct AppAssets;

impl AssetSource for AppAssets {
  fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
    if let Some(file) = Embedded::get(path) {
      return Ok(Some(file.data));
    }
    gpui_kit::assets::Assets.load(path)
  }

  fn list(&self, path: &str) -> Result<Vec<SharedString>> {
    let mut names: Vec<SharedString> = Embedded::iter()
      .filter(|name| name.starts_with(path))
      .map(|name| SharedString::from(name.to_string()))
      .collect();
    names.extend(gpui_kit::assets::Assets.list(path)?);
    Ok(names)
  }
}

/// Font bytes for `TextSystem::add_fonts`.
pub fn font_files() -> Vec<Cow<'static, [u8]>> {
  Embedded::iter()
    .filter(|name| name.starts_with("fonts/") && name.ends_with(".ttf"))
    .filter_map(|name| Embedded::get(&name).map(|file| file.data))
    .collect()
}

/// Every bundled theme as `(id, json)`.
pub fn theme_files() -> Vec<(String, String)> {
  let mut files: Vec<(String, String)> = Embedded::iter()
    .filter(|name| name.starts_with("themes/") && name.ends_with(".json"))
    .filter_map(|name| {
      let id = name.trim_start_matches("themes/").trim_end_matches(".json").to_string();
      let data = Embedded::get(&name)?.data;
      Some((id, String::from_utf8_lossy(&data).into_owned()))
    })
    .collect();
  files.sort_by(|a, b| a.0.cmp(&b.0));
  files
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn embeds_themes_icons_fonts_and_brand() {
    assert!(Embedded::get("themes/vesper.json").is_some());
    assert!(Embedded::get("themes/ayu-light.json").is_some());
    assert!(Embedded::get("icons/folder.svg").is_some());
    assert!(Embedded::get("brand/deathpush.svg").is_some());
    assert_eq!(font_files().len(), 4);
  }

  #[test]
  fn falls_back_to_gpui_kit_icons() {
    let listed = AppAssets.list("icons/").unwrap();
    assert!(listed.iter().any(|name| name.ends_with("folder.svg")));
  }

  #[test]
  fn lists_every_bundled_theme() {
    let files = theme_files();
    assert_eq!(files.len(), 65);
    assert!(files.iter().any(|(id, _)| id == "vesper"));
  }
}
