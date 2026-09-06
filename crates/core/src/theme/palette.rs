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
  pub git_added: Rgba,
  pub git_modified: Rgba,
  pub git_deleted: Rgba,
  pub git_renamed: Rgba,
  pub git_untracked: Rgba,
  pub git_ignored: Rgba,
  pub git_conflicting: Rgba,
  pub git_staged_modified: Rgba,
  pub git_staged_deleted: Rgba,
  pub diff_inserted_line: Rgba,
  pub diff_removed_line: Rgba,
  pub diff_inserted_text: Rgba,
  pub diff_removed_text: Rgba,
  pub gutter_added: Rgba,
  pub gutter_modified: Rgba,
  pub gutter_deleted: Rgba,
  pub terminal_background: Rgba,
  pub terminal_foreground: Rgba,
  pub terminal_cursor: Rgba,
  pub terminal_ansi: [Rgba; 16],
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
    let muted_foreground = first(spec, &["descriptionForeground"]).unwrap_or(foreground.with_alpha(180));
    let git_added = first(spec, &["gitDecoration.addedResourceForeground"]).unwrap_or(Rgba::rgb(129, 184, 139));
    let git_modified = first(spec, &["gitDecoration.modifiedResourceForeground"]).unwrap_or(Rgba::rgb(226, 192, 141));
    let git_deleted = first(spec, &["gitDecoration.deletedResourceForeground"]).unwrap_or(Rgba::rgb(200, 116, 112));
    let git_renamed = first(spec, &["gitDecoration.renamedResourceForeground"]).unwrap_or(git_added);
    let git_untracked = first(spec, &["gitDecoration.untrackedResourceForeground"]).unwrap_or(Rgba::rgb(115, 201, 145));
    let git_ignored = first(spec, &["gitDecoration.ignoredResourceForeground"]).unwrap_or(muted_foreground);
    let git_conflicting =
      first(spec, &["gitDecoration.conflictingResourceForeground"]).unwrap_or(Rgba::rgb(229, 148, 0));
    let git_staged_modified = first(spec, &["gitDecoration.stageModifiedResourceForeground"]).unwrap_or(git_modified);
    let git_staged_deleted = first(spec, &["gitDecoration.stageDeletedResourceForeground"]).unwrap_or(git_deleted);
    let diff_inserted_line = first(spec, &["diffEditor.insertedLineBackground"]).unwrap_or(git_added.with_alpha(38));
    let diff_removed_line = first(spec, &["diffEditor.removedLineBackground"]).unwrap_or(git_deleted.with_alpha(38));
    let diff_inserted_text = first(spec, &["diffEditor.insertedTextBackground"]).unwrap_or(git_added.with_alpha(90));
    let diff_removed_text = first(spec, &["diffEditor.removedTextBackground"]).unwrap_or(git_deleted.with_alpha(90));
    let gutter_added = first(spec, &["editorGutter.addedBackground"]).unwrap_or(git_added);
    let gutter_modified = first(spec, &["editorGutter.modifiedBackground"]).unwrap_or(git_modified);
    let gutter_deleted = first(spec, &["editorGutter.deletedBackground"]).unwrap_or(git_deleted);
    let caret = first(spec, &["editorCursor.foreground"]).unwrap_or(foreground);
    let terminal_background = first(spec, &["terminal.background"]).unwrap_or(background);
    let terminal_foreground = first(spec, &["terminal.foreground"]).unwrap_or(foreground);
    let terminal_cursor = first(spec, &["terminalCursor.foreground"]).unwrap_or(caret);
    let terminal_ansi = ansi_palette(spec, dark);
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
      muted_foreground,
      accent: list_hover,
      input,
      input_border: first(spec, &["input.border"]).unwrap_or(border),
      ring: first(spec, &["focusBorder"]).unwrap_or(primary),
      caret,
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
      git_added,
      git_modified,
      git_deleted,
      git_renamed,
      git_untracked,
      git_ignored,
      git_conflicting,
      git_staged_modified,
      git_staged_deleted,
      diff_inserted_line,
      diff_removed_line,
      diff_inserted_text,
      diff_removed_text,
      gutter_added,
      gutter_modified,
      gutter_deleted,
      terminal_background,
      terminal_foreground,
      terminal_cursor,
      terminal_ansi,
    }
  }
}

const ANSI_KEYS: [&str; 16] = [
  "terminal.ansiBlack",
  "terminal.ansiRed",
  "terminal.ansiGreen",
  "terminal.ansiYellow",
  "terminal.ansiBlue",
  "terminal.ansiMagenta",
  "terminal.ansiCyan",
  "terminal.ansiWhite",
  "terminal.ansiBrightBlack",
  "terminal.ansiBrightRed",
  "terminal.ansiBrightGreen",
  "terminal.ansiBrightYellow",
  "terminal.ansiBrightBlue",
  "terminal.ansiBrightMagenta",
  "terminal.ansiBrightCyan",
  "terminal.ansiBrightWhite",
];

const ANSI_DARK: [Rgba; 16] = [
  Rgba::rgb(0x00, 0x00, 0x00),
  Rgba::rgb(0xcd, 0x31, 0x31),
  Rgba::rgb(0x0d, 0xbc, 0x79),
  Rgba::rgb(0xe5, 0xe5, 0x10),
  Rgba::rgb(0x24, 0x72, 0xc8),
  Rgba::rgb(0xbc, 0x3f, 0xbc),
  Rgba::rgb(0x11, 0xa8, 0xcd),
  Rgba::rgb(0xe5, 0xe5, 0xe5),
  Rgba::rgb(0x66, 0x66, 0x66),
  Rgba::rgb(0xf1, 0x4c, 0x4c),
  Rgba::rgb(0x23, 0xd1, 0x8b),
  Rgba::rgb(0xf5, 0xf5, 0x43),
  Rgba::rgb(0x3b, 0x8e, 0xea),
  Rgba::rgb(0xd6, 0x70, 0xd6),
  Rgba::rgb(0x29, 0xb8, 0xdb),
  Rgba::rgb(0xe5, 0xe5, 0xe5),
];

const ANSI_LIGHT: [Rgba; 16] = [
  Rgba::rgb(0x00, 0x00, 0x00),
  Rgba::rgb(0xcd, 0x31, 0x31),
  Rgba::rgb(0x00, 0xbc, 0x00),
  Rgba::rgb(0x94, 0x98, 0x00),
  Rgba::rgb(0x04, 0x51, 0xa5),
  Rgba::rgb(0xbc, 0x05, 0xbc),
  Rgba::rgb(0x05, 0x98, 0xbc),
  Rgba::rgb(0x55, 0x55, 0x55),
  Rgba::rgb(0x66, 0x66, 0x66),
  Rgba::rgb(0xcd, 0x31, 0x31),
  Rgba::rgb(0x14, 0xce, 0x14),
  Rgba::rgb(0xb5, 0xba, 0x00),
  Rgba::rgb(0x04, 0x51, 0xa5),
  Rgba::rgb(0xbc, 0x05, 0xbc),
  Rgba::rgb(0x05, 0x98, 0xbc),
  Rgba::rgb(0xa5, 0xa5, 0xa5),
];

fn ansi_palette(spec: &ThemeSpec, dark: bool) -> [Rgba; 16] {
  let fallbacks = if dark { ANSI_DARK } else { ANSI_LIGHT };
  let mut colors = fallbacks;
  for (index, key) in ANSI_KEYS.iter().enumerate() {
    if let Some(color) = spec.color(key) {
      colors[index] = color;
    }
  }
  colors
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

  #[test]
  fn git_and_diff_colors_fall_back_when_the_theme_has_no_keys() {
    let spec = parse_theme(r##"{"name":"t","type":"dark","colors":{"editor.background":"#101010"}}"##).unwrap();
    let palette = UiPalette::from_spec(&spec);
    assert_eq!(palette.git_added, Rgba::rgb(129, 184, 139));
    assert_eq!(palette.git_modified, Rgba::rgb(226, 192, 141));
    assert_eq!(palette.git_deleted, Rgba::rgb(200, 116, 112));
    assert_eq!(palette.git_untracked, Rgba::rgb(115, 201, 145));
    assert_eq!(palette.git_conflicting, Rgba::rgb(229, 148, 0));
    assert_eq!(palette.git_renamed, palette.git_added);
    assert_eq!(palette.git_staged_modified, palette.git_modified);
    assert_eq!(palette.git_staged_deleted, palette.git_deleted);
    assert_eq!(palette.git_ignored, palette.muted_foreground);
    assert_eq!(palette.diff_inserted_line, palette.git_added.with_alpha(38));
    assert_eq!(palette.diff_removed_line, palette.git_deleted.with_alpha(38));
    assert_eq!(palette.diff_inserted_text, palette.git_added.with_alpha(90));
    assert_eq!(palette.diff_removed_text, palette.git_deleted.with_alpha(90));
    assert_eq!(palette.gutter_added, palette.git_added);
    assert_eq!(palette.gutter_modified, palette.git_modified);
    assert_eq!(palette.gutter_deleted, palette.git_deleted);
  }

  #[test]
  fn git_and_diff_colors_read_the_theme_keys() {
    let spec = parse_theme(
      r##"{"name":"t","type":"dark","colors":{
      "gitDecoration.addedResourceForeground":"#11aa11",
      "gitDecoration.modifiedResourceForeground":"#aaaa11",
      "gitDecoration.deletedResourceForeground":"#aa1111",
      "gitDecoration.renamedResourceForeground":"#1111aa",
      "gitDecoration.untrackedResourceForeground":"#11aaaa",
      "gitDecoration.ignoredResourceForeground":"#777777",
      "gitDecoration.conflictingResourceForeground":"#aa11aa",
      "gitDecoration.stageModifiedResourceForeground":"#bbbb22",
      "gitDecoration.stageDeletedResourceForeground":"#bb2222",
      "diffEditor.insertedLineBackground":"#11aa1122",
      "diffEditor.removedLineBackground":"#aa111122",
      "diffEditor.insertedTextBackground":"#11aa1155",
      "diffEditor.removedTextBackground":"#aa111155",
      "editorGutter.addedBackground":"#22bb22",
      "editorGutter.modifiedBackground":"#bbbb22",
      "editorGutter.deletedBackground":"#bb2222"
    }}"##,
    )
    .unwrap();
    let palette = UiPalette::from_spec(&spec);
    assert_eq!(palette.git_added, Rgba::rgb(0x11, 0xaa, 0x11));
    assert_eq!(palette.git_renamed, Rgba::rgb(0x11, 0x11, 0xaa));
    assert_eq!(palette.git_ignored, Rgba::rgb(0x77, 0x77, 0x77));
    assert_eq!(palette.git_staged_deleted, Rgba::rgb(0xbb, 0x22, 0x22));
    assert_eq!(palette.diff_inserted_line, Rgba::rgb(0x11, 0xaa, 0x11).with_alpha(0x22));
    assert_eq!(palette.diff_removed_text, Rgba::rgb(0xaa, 0x11, 0x11).with_alpha(0x55));
    assert_eq!(palette.gutter_deleted, Rgba::rgb(0xbb, 0x22, 0x22));
  }

  #[test]
  fn palette_terminal_colors_fall_back() {
    let dark = parse_theme(r##"{"name":"t","type":"dark","colors":{"editor.background":"#101010","editor.foreground":"#eeeeee","editorCursor.foreground":"#00ff00"}}"##).unwrap();
    let dark_palette = UiPalette::from_spec(&dark);
    assert_eq!(dark_palette.terminal_background, dark_palette.background);
    assert_eq!(dark_palette.terminal_foreground, dark_palette.foreground);
    assert_eq!(dark_palette.terminal_cursor, dark_palette.caret);
    assert_eq!(
      dark_palette.terminal_ansi,
      [
        Rgba::rgb(0x00, 0x00, 0x00),
        Rgba::rgb(0xcd, 0x31, 0x31),
        Rgba::rgb(0x0d, 0xbc, 0x79),
        Rgba::rgb(0xe5, 0xe5, 0x10),
        Rgba::rgb(0x24, 0x72, 0xc8),
        Rgba::rgb(0xbc, 0x3f, 0xbc),
        Rgba::rgb(0x11, 0xa8, 0xcd),
        Rgba::rgb(0xe5, 0xe5, 0xe5),
        Rgba::rgb(0x66, 0x66, 0x66),
        Rgba::rgb(0xf1, 0x4c, 0x4c),
        Rgba::rgb(0x23, 0xd1, 0x8b),
        Rgba::rgb(0xf5, 0xf5, 0x43),
        Rgba::rgb(0x3b, 0x8e, 0xea),
        Rgba::rgb(0xd6, 0x70, 0xd6),
        Rgba::rgb(0x29, 0xb8, 0xdb),
        Rgba::rgb(0xe5, 0xe5, 0xe5),
      ]
    );

    let light = parse_theme(r##"{"name":"t","type":"light","colors":{}}"##).unwrap();
    let light_palette = UiPalette::from_spec(&light);
    assert_eq!(light_palette.terminal_background, light_palette.background);
    assert_eq!(light_palette.terminal_foreground, light_palette.foreground);
    assert_eq!(light_palette.terminal_cursor, light_palette.caret);
    assert_eq!(
      light_palette.terminal_ansi,
      [
        Rgba::rgb(0x00, 0x00, 0x00),
        Rgba::rgb(0xcd, 0x31, 0x31),
        Rgba::rgb(0x00, 0xbc, 0x00),
        Rgba::rgb(0x94, 0x98, 0x00),
        Rgba::rgb(0x04, 0x51, 0xa5),
        Rgba::rgb(0xbc, 0x05, 0xbc),
        Rgba::rgb(0x05, 0x98, 0xbc),
        Rgba::rgb(0x55, 0x55, 0x55),
        Rgba::rgb(0x66, 0x66, 0x66),
        Rgba::rgb(0xcd, 0x31, 0x31),
        Rgba::rgb(0x14, 0xce, 0x14),
        Rgba::rgb(0xb5, 0xba, 0x00),
        Rgba::rgb(0x04, 0x51, 0xa5),
        Rgba::rgb(0xbc, 0x05, 0xbc),
        Rgba::rgb(0x05, 0x98, 0xbc),
        Rgba::rgb(0xa5, 0xa5, 0xa5),
      ]
    );

    let keyed = parse_theme(
      r##"{"name":"t","type":"dark","colors":{
      "terminal.background":"#111111",
      "terminal.foreground":"#eeeeee",
      "terminalCursor.foreground":"#00ffaa",
      "terminal.ansiBlack":"#010101",
      "terminal.ansiRed":"#020202",
      "terminal.ansiGreen":"#030303",
      "terminal.ansiYellow":"#040404",
      "terminal.ansiBlue":"#050505",
      "terminal.ansiMagenta":"#060606",
      "terminal.ansiCyan":"#070707",
      "terminal.ansiWhite":"#080808",
      "terminal.ansiBrightBlack":"#090909",
      "terminal.ansiBrightRed":"#0a0a0a",
      "terminal.ansiBrightGreen":"#0b0b0b",
      "terminal.ansiBrightYellow":"#0c0c0c",
      "terminal.ansiBrightBlue":"#0d0d0d",
      "terminal.ansiBrightMagenta":"#0e0e0e",
      "terminal.ansiBrightCyan":"#0f0f0f",
      "terminal.ansiBrightWhite":"#101010"
    }}"##,
    )
    .unwrap();
    let keyed_palette = UiPalette::from_spec(&keyed);
    assert_eq!(keyed_palette.terminal_background, Rgba::rgb(0x11, 0x11, 0x11));
    assert_eq!(keyed_palette.terminal_foreground, Rgba::rgb(0xee, 0xee, 0xee));
    assert_eq!(keyed_palette.terminal_cursor, Rgba::rgb(0x00, 0xff, 0xaa));
    assert_eq!(keyed_palette.terminal_ansi[0], Rgba::rgb(0x01, 0x01, 0x01));
    assert_eq!(keyed_palette.terminal_ansi[8], Rgba::rgb(0x09, 0x09, 0x09));
    assert_eq!(keyed_palette.terminal_ansi[15], Rgba::rgb(0x10, 0x10, 0x10));
  }
}
