use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

use deathpush_core::config::settings::{DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME};
use deathpush_core::theme::{Rgba, ThemeKind, ThemeSpec, UiPalette, parse_theme};
use gpui_kit::component::theme::{Theme, ThemeConfig, ThemeConfigColors, ThemeMode, ThemeRegistry, ThemeSet};
use gpui_kit::*;

use crate::assets;
use crate::config::AppConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeEntry {
  pub id: String,
  pub label: String,
  pub kind: ThemeKind,
}

/// Every bundled theme, parsed once.
pub struct ThemeCatalog {
  #[allow(dead_code)]
  pub entries: Vec<ThemeEntry>,
  specs: HashMap<String, Arc<ThemeSpec>>,
  palettes: HashMap<String, UiPalette>,
}

impl Global for ThemeCatalog {}

impl ThemeCatalog {
  pub fn get(cx: &App) -> &Self {
    cx.global::<Self>()
  }

  pub fn palette(&self, id: &str) -> Option<UiPalette> {
    self.palettes.get(id).copied()
  }

  #[allow(dead_code)]
  pub fn spec(&self, id: &str) -> Option<Arc<ThemeSpec>> {
    self.specs.get(id).cloned()
  }

  pub fn kind(&self, id: &str) -> Option<ThemeKind> {
    self.specs.get(id).map(|spec| spec.kind)
  }
}

/// The palette of the theme in effect, for elements gpui-component does not paint (mark, overlay, badge, status bar text).
#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct ActivePalette(pub UiPalette);

impl Global for ActivePalette {}

#[allow(dead_code)]
pub fn hsla(color: Rgba) -> Hsla {
  Rgba {
    r: color.r,
    g: color.g,
    b: color.b,
    a: color.a,
  }
  .into_gpui()
}

#[allow(dead_code)]
trait IntoGpui {
  fn into_gpui(self) -> Hsla;
}

impl IntoGpui for Rgba {
  fn into_gpui(self) -> Hsla {
    gpui_kit::Rgba {
      r: self.r as f32 / 255.0,
      g: self.g as f32 / 255.0,
      b: self.b as f32 / 255.0,
      a: self.a as f32 / 255.0,
    }
    .into()
  }
}

fn hex(color: Rgba) -> Option<SharedString> {
  Some(color.to_hex().into())
}

/// gpui-component's theme config for one of our themes.
pub fn theme_config(spec: &ThemeSpec, palette: &UiPalette, ui_font_family: &str, ui_font_size: u32) -> ThemeConfig {
  // Base.* fields on ThemeConfigColors are private, so struct update from outside the crate does not compile.
  let mut colors = ThemeConfigColors::default();
  colors.background = hex(palette.background);
  colors.foreground = hex(palette.foreground);
  colors.sidebar = hex(palette.sidebar);
  colors.sidebar_foreground = hex(palette.sidebar_foreground);
  colors.sidebar_border = hex(palette.sidebar_border);
  colors.sidebar_accent = hex(palette.list_hover);
  colors.sidebar_accent_foreground = hex(palette.foreground);
  colors.sidebar_primary = hex(palette.primary);
  colors.sidebar_primary_foreground = hex(palette.primary_foreground);
  colors.title_bar = hex(palette.title_bar);
  colors.title_bar_border = hex(palette.border);
  colors.status_bar = hex(palette.status_bar);
  colors.status_bar_border = hex(palette.border);
  colors.border = hex(palette.border);
  colors.primary = hex(palette.primary);
  colors.primary_foreground = hex(palette.primary_foreground);
  colors.primary_hover = hex(palette.primary_hover);
  colors.primary_active = hex(palette.primary_hover);
  colors.secondary = hex(palette.secondary);
  colors.secondary_foreground = hex(palette.secondary_foreground);
  colors.secondary_hover = hex(palette.secondary_hover);
  colors.secondary_active = hex(palette.secondary_hover);
  colors.muted = hex(palette.muted);
  colors.muted_foreground = hex(palette.muted_foreground);
  colors.accent = hex(palette.accent);
  colors.accent_foreground = hex(palette.foreground);
  colors.input = hex(palette.input_border);
  colors.ring = hex(palette.ring);
  colors.caret = hex(palette.caret);
  colors.list = hex(palette.sidebar);
  colors.list_active = hex(palette.list_active);
  colors.list_active_border = hex(palette.ring);
  colors.list_hover = hex(palette.list_hover);
  colors.list_even = hex(palette.sidebar);
  colors.list_head = hex(palette.sidebar);
  colors.popover = hex(palette.popover);
  colors.popover_foreground = hex(palette.popover_foreground);
  colors.selection = hex(palette.selection);
  colors.link = hex(palette.link);
  colors.link_hover = hex(palette.link_hover);
  colors.link_active = hex(palette.link_hover);
  colors.danger = hex(palette.danger);
  colors.danger_foreground = hex(Rgba::rgb(255, 255, 255));
  colors.warning = hex(palette.warning);
  colors.success = hex(palette.success);
  colors.info = hex(palette.info);
  colors.scrollbar = hex(palette.background.with_alpha(0));
  colors.scrollbar_thumb = hex(palette.scrollbar_thumb);
  colors.scrollbar_thumb_hover = hex(palette.scrollbar_thumb_hover);
  colors.tab_bar = hex(palette.sidebar);
  colors.tab = hex(palette.sidebar);
  colors.tab_active = hex(palette.background);
  colors.tab_foreground = hex(palette.muted_foreground);
  colors.tab_active_foreground = hex(palette.foreground);
  colors.overlay = hex(palette.overlay);
  colors.window_border = hex(palette.border);
  ThemeConfig {
    name: spec.name.clone().into(),
    mode: if spec.kind == ThemeKind::Dark {
      ThemeMode::Dark
    } else {
      ThemeMode::Light
    },
    font_family: (!ui_font_family.is_empty()).then(|| ui_font_family.to_string().into()),
    font_size: Some(ui_font_size as f32),
    colors,
    ..Default::default()
  }
}

/// Parse every bundled theme, register them with gpui-component, and apply the saved theme.
pub fn init(cx: &mut App) {
  let mut entries = Vec::new();
  let mut specs = HashMap::new();
  let mut palettes = HashMap::new();
  let (font_family, font_size) = {
    let ui = &AppConfig::get(cx).settings.ui;
    (ui.font_family.clone(), ui.font_size)
  };
  let mut configs = Vec::new();
  for (id, json) in assets::theme_files() {
    let spec = match parse_theme(&json) {
      Ok(spec) => spec,
      Err(err) => {
        tracing::warn!("skipping theme {id}: {err}");
        continue;
      }
    };
    let palette = UiPalette::from_spec(&spec);
    configs.push(theme_config(&spec, &palette, &font_family, font_size));
    entries.push(ThemeEntry {
      id: id.clone(),
      label: spec.label(),
      kind: spec.kind,
    });
    palettes.insert(id.clone(), palette);
    specs.insert(id, Arc::new(spec));
  }
  entries.sort_by_key(|a| a.label.to_lowercase());
  let set = ThemeSet {
    name: "DeathPush".into(),
    themes: configs,
    ..Default::default()
  };
  let json = serde_json::to_string(&set).expect("theme set serializes");
  ThemeRegistry::global_mut(cx)
    .load_themes_from_str(&json)
    .expect("theme set registers");
  cx.set_global(ThemeCatalog {
    entries,
    specs,
    palettes,
  });
  let current = AppConfig::get(cx).settings.theme.current.clone();
  let wanted = ThemeCatalog::get(cx).kind(&current).unwrap_or(ThemeKind::Dark);
  apply_theme(&current, wanted, None, cx);
}

fn resolve_id(catalog: &ThemeCatalog, id: &str, wanted: ThemeKind) -> String {
  if catalog.specs.contains_key(id) {
    id.to_string()
  } else {
    default_for(wanted).to_string()
  }
}

/// Switch gpui-component and our palette to `id`. Unknown ids fall back to the default of their kind.
pub fn apply_theme(id: &str, wanted: ThemeKind, window: Option<&mut Window>, cx: &mut App) {
  let catalog = ThemeCatalog::get(cx);
  let id = resolve_id(catalog, id, wanted);
  let kind = catalog.kind(&id).unwrap_or(ThemeKind::Dark);
  let palette = catalog.palette(&id).expect("catalog has the id");
  let config: Rc<ThemeConfig> = ThemeRegistry::global(cx)
    .themes()
    .get(&SharedString::from(id.clone()))
    .cloned()
    .expect("registry has the theme");
  {
    let theme = Theme::global_mut(cx);
    if kind == ThemeKind::Dark {
      theme.dark_theme = config;
    } else {
      theme.light_theme = config;
    }
  }
  Theme::change(
    if kind == ThemeKind::Dark {
      ThemeMode::Dark
    } else {
      ThemeMode::Light
    },
    window,
    cx,
  );
  cx.set_global(ActivePalette(palette));
  AppConfig::update(cx, |c| c.settings.theme.current = id);
}

/// The preferred theme for the OS appearance.
#[allow(dead_code)]
pub fn apply_for_appearance(appearance: WindowAppearance, window: Option<&mut Window>, cx: &mut App) {
  let dark = matches!(appearance, WindowAppearance::Dark | WindowAppearance::VibrantDark);
  let wanted = if dark { ThemeKind::Dark } else { ThemeKind::Light };
  let id = {
    let theme = &AppConfig::get(cx).settings.theme;
    if dark {
      theme.preferred_dark.clone()
    } else {
      theme.preferred_light.clone()
    }
  };
  apply_theme(&id, wanted, window, cx);
}

pub fn default_for(kind: ThemeKind) -> &'static str {
  match kind {
    ThemeKind::Dark => DEFAULT_DARK_THEME,
    ThemeKind::Light => DEFAULT_LIGHT_THEME,
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  #[test]
  fn config_carries_the_palette_and_mode() {
    let spec = parse_theme(include_str!("../../../assets/themes/ayu-light.json")).unwrap();
    let palette = UiPalette::from_spec(&spec);
    let config = theme_config(&spec, &palette, "", 13);
    assert_eq!(config.mode, ThemeMode::Light);
    assert_eq!(
      config.colors.background.as_deref(),
      Some(palette.background.to_hex().as_str())
    );
    assert!(config.font_family.is_none());
    let set = ThemeSet {
      name: "t".into(),
      themes: vec![config],
      ..Default::default()
    };
    serde_json::to_string(&set).unwrap();
  }

  #[gpui_kit::test]
  fn init_registers_every_theme_and_applies_the_saved_one(cx: &mut TestAppContext) {
    let dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(dir.path().to_path_buf(), cx);
      init(cx);
      assert_eq!(ThemeCatalog::get(cx).entries.len(), 65);
      assert_eq!(Theme::global(cx).mode, ThemeMode::Dark);
      apply_theme("ayu-light", ThemeKind::Light, None, cx);
      assert_eq!(Theme::global(cx).mode, ThemeMode::Light);
      assert_eq!(cx.global::<ActivePalette>().0.kind, ThemeKind::Light);
    });
  }
}
