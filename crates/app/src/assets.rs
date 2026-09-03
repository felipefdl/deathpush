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
}
