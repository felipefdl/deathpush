use super::spec::{Rgba, ThemeKind, ThemeSpec};

/// Role colors for the app chrome, resolved from a theme's `colors` with fallbacks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UiPalette {
  pub kind: ThemeKind,
  pub background: Rgba,
  pub foreground: Rgba,
  pub sidebar: Rgba,
  pub sidebar_foreground: Rgba,
  pub sidebar_border: Rgba,
  pub title_bar: Rgba,
  pub title_bar_foreground: Rgba,
  pub status_bar: Rgba,
  pub status_bar_foreground: Rgba,
  pub border: Rgba,
  pub primary: Rgba,
  pub primary_foreground: Rgba,
  pub primary_hover: Rgba,
  pub secondary: Rgba,
  pub secondary_foreground: Rgba,
  pub secondary_hover: Rgba,
  pub muted: Rgba,
  pub muted_foreground: Rgba,
  pub accent: Rgba,
  pub input: Rgba,
  pub input_border: Rgba,
  pub ring: Rgba,
  pub caret: Rgba,
  pub list_active: Rgba,
  pub list_active_foreground: Rgba,
  pub list_hover: Rgba,
  pub list_inactive: Rgba,
  pub popover: Rgba,
  pub popover_foreground: Rgba,
  pub popover_border: Rgba,
  pub selection: Rgba,
  pub link: Rgba,
  pub link_hover: Rgba,
  pub danger: Rgba,
  pub warning: Rgba,
  pub success: Rgba,
  pub info: Rgba,
  pub scrollbar_thumb: Rgba,
  pub scrollbar_thumb_hover: Rgba,
  pub badge: Rgba,
  pub badge_foreground: Rgba,
  pub overlay: Rgba,
  /// The app mark: white on dark themes, black on light themes.
  pub mark: Rgba,
}

fn first(spec: &ThemeSpec, keys: &[&str]) -> Option<Rgba> {
  keys.iter().find_map(|key| spec.color(key))
}

impl UiPalette {
  pub fn from_spec(spec: &ThemeSpec) -> Self {
    let dark = spec.kind == ThemeKind::Dark;
    let background = first(spec, &["editor.background"]).unwrap_or(if dark {
      Rgba::rgb(30, 30, 30)
    } else {
      Rgba::rgb(255, 255, 255)
    });
    let foreground = first(spec, &["editor.foreground", "foreground"]).unwrap_or(if dark {
      Rgba::rgb(212, 212, 212)
    } else {
      Rgba::rgb(51, 51, 51)
    });
    let hairline = foreground.with_alpha(38);
    let hover = foreground.with_alpha(20);
    let sidebar = first(spec, &["sideBar.background"]).unwrap_or(background);
    let sidebar_foreground = first(spec, &["sideBar.foreground"]).unwrap_or(foreground);
    let border = first(
      spec,
      &["panel.border", "editorGroup.border", "widget.border", "contrastBorder"],
    )
    .unwrap_or(hairline);
    let primary = first(spec, &["button.background"]).unwrap_or(if dark {
      Rgba::rgb(14, 99, 156)
    } else {
      Rgba::rgb(0, 122, 204)
    });
    let primary_foreground = first(spec, &["button.foreground"]).unwrap_or(Rgba::rgb(255, 255, 255));
    let primary_hover = first(spec, &["button.hoverBackground"]).unwrap_or(primary.mix(foreground, 0.15));
    let input = first(spec, &["input.background"]).unwrap_or(sidebar.mix(foreground, 0.05));
    let secondary = first(spec, &["button.secondaryBackground"]).unwrap_or(input);
    let secondary_foreground = first(spec, &["button.secondaryForeground"]).unwrap_or(foreground);
    let secondary_hover = first(spec, &["button.secondaryHoverBackground"]).unwrap_or(secondary.mix(foreground, 0.1));
    let list_active = first(spec, &["list.activeSelectionBackground"]).unwrap_or(primary.with_alpha(120));
    let list_hover = first(spec, &["list.hoverBackground"]).unwrap_or(hover);
    let popover = first(
      spec,
      &["menu.background", "editorWidget.background", "dropdown.background"],
    )
    .unwrap_or(sidebar);
    let danger = first(
      spec,
      &["errorForeground", "list.errorForeground", "editorError.foreground"],
    )
    .unwrap_or(Rgba::rgb(241, 76, 76));
    let badge = first(spec, &["badge.background"]).unwrap_or(primary);
    Self {
      kind: spec.kind,
      background,
      foreground,
      sidebar,
      sidebar_foreground,
      sidebar_border: first(spec, &["sideBar.border"]).unwrap_or(border),
      title_bar: first(spec, &["titleBar.activeBackground"]).unwrap_or(sidebar),
      title_bar_foreground: first(spec, &["titleBar.activeForeground"]).unwrap_or(sidebar_foreground),
      status_bar: first(spec, &["statusBar.background"]).unwrap_or(sidebar),
      status_bar_foreground: first(spec, &["statusBar.foreground"]).unwrap_or(sidebar_foreground),
      border,
      primary,
      primary_foreground,
      primary_hover,
      secondary,
      secondary_foreground,
      secondary_hover,
      muted: input,
      muted_foreground: first(spec, &["descriptionForeground"]).unwrap_or(foreground.with_alpha(180)),
      accent: list_hover,
      input,
      input_border: first(spec, &["input.border"]).unwrap_or(border),
      ring: first(spec, &["focusBorder"]).unwrap_or(primary),
      caret: first(spec, &["editorCursor.foreground"]).unwrap_or(foreground),
      list_active,
      list_active_foreground: first(spec, &["list.activeSelectionForeground"]).unwrap_or(foreground),
      list_hover,
      list_inactive: first(spec, &["list.inactiveSelectionBackground"]).unwrap_or(list_active.with_alpha(90)),
      popover,
      popover_foreground: first(spec, &["menu.foreground", "editorWidget.foreground"]).unwrap_or(foreground),
      popover_border: first(spec, &["menu.border", "editorWidget.border"]).unwrap_or(border),
      selection: first(spec, &["editor.selectionBackground"]).unwrap_or(primary.with_alpha(90)),
      link: first(spec, &["textLink.foreground"]).unwrap_or(primary),
      link_hover: first(spec, &["textLink.activeForeground"]).unwrap_or(primary_hover),
      danger,
      warning: first(spec, &["editorWarning.foreground", "list.warningForeground"]).unwrap_or(Rgba::rgb(204, 167, 0)),
      success: first(spec, &["gitDecoration.addedResourceForeground", "terminal.ansiGreen"])
        .unwrap_or(Rgba::rgb(137, 209, 133)),
      info: first(spec, &["editorInfo.foreground", "terminal.ansiBlue"]).unwrap_or(Rgba::rgb(55, 148, 255)),
      scrollbar_thumb: first(spec, &["scrollbarSlider.background"]).unwrap_or(foreground.with_alpha(60)),
      scrollbar_thumb_hover: first(spec, &["scrollbarSlider.hoverBackground"]).unwrap_or(foreground.with_alpha(100)),
      badge,
      badge_foreground: first(spec, &["badge.foreground"]).unwrap_or(primary_foreground),
      overlay: Rgba {
        r: 0,
        g: 0,
        b: 0,
        a: 102,
      },
      mark: if dark {
        Rgba::rgb(255, 255, 255)
      } else {
        Rgba::rgb(0, 0, 0)
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::theme::parse_theme;

  #[test]
  fn vesper_maps_its_own_colors() {
    let spec = parse_theme(include_str!("../../../../assets/themes/vesper.json")).unwrap();
    let palette = UiPalette::from_spec(&spec);
    assert_eq!(Some(palette.background), spec.color("editor.background"));
    assert_eq!(Some(palette.sidebar), spec.color("sideBar.background"));
    assert_eq!(Some(palette.primary), spec.color("button.background"));
    assert_eq!(palette.mark, Rgba::rgb(255, 255, 255));
    assert_eq!(palette.kind, ThemeKind::Dark);
  }

  #[test]
  fn missing_keys_fall_back_without_panicking() {
    let spec = ThemeSpec {
      name: "bare".into(),
      display_name: None,
      kind: ThemeKind::Light,
      colors: Default::default(),
      token_colors: vec![],
    };
    let palette = UiPalette::from_spec(&spec);
    assert_eq!(palette.background, Rgba::rgb(255, 255, 255));
    assert_eq!(palette.sidebar, palette.background);
    assert_eq!(palette.mark, Rgba::rgb(0, 0, 0));
    assert!(palette.border.a < 255);
  }

  #[test]
  fn every_bundled_theme_yields_a_palette() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/themes");
    for entry in std::fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      if path.extension().is_some_and(|ext| ext == "json") {
        let spec = parse_theme(&std::fs::read_to_string(&path).unwrap()).unwrap();
        let palette = UiPalette::from_spec(&spec);
        assert_ne!(palette.background, palette.foreground, "{}", path.display());
      }
    }
  }
}
