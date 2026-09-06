use super::spec::{Rgba, ThemeKind, ThemeSpec, ThemeStyle};

/// Role colors for the app chrome, resolved from a Zed theme's `style`.
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

/// Keys are read from the theme first, then from the base theme of the same appearance.
/// Within one call the whole alias list is tried against the theme before the base is touched,
/// so a theme's own sibling color always beats the base theme's exact key.
struct Resolver<'a> {
  theme: &'a ThemeStyle,
  base: &'a ThemeStyle,
}

impl Resolver<'_> {
  fn pick(&self, keys: &[&str]) -> Option<Rgba> {
    keys
      .iter()
      .find_map(|key| self.theme.color(key))
      .or_else(|| keys.iter().find_map(|key| self.base.color(key)))
  }

  fn cursor(&self) -> Option<Rgba> {
    self.theme.cursor().or_else(|| self.base.cursor())
  }

  fn selection(&self) -> Option<Rgba> {
    self.theme.selection().or_else(|| self.base.selection())
  }
}

impl UiPalette {
  /// Every role for `spec`, with `base` (One Dark or One Light) behind it.
  pub fn resolve(spec: &ThemeSpec, base: &ThemeStyle) -> Self {
    let style = Resolver {
      theme: &spec.style,
      base,
    };
    let dark = spec.kind.is_dark();
    let background = style.pick(&["editor.background", "background"]).unwrap_or(if dark {
      Rgba::rgb(30, 30, 30)
    } else {
      Rgba::rgb(255, 255, 255)
    });
    let foreground = style.pick(&["editor.foreground", "text"]).unwrap_or(if dark {
      Rgba::rgb(212, 212, 212)
    } else {
      Rgba::rgb(51, 51, 51)
    });
    let hairline = foreground.with_alpha(38);
    let hover = foreground.with_alpha(20);
    let border = style.pick(&["border", "border.variant"]).unwrap_or(hairline);
    let sidebar = style
      .pick(&["panel.background", "surface.background"])
      .unwrap_or(background);
    let sidebar_foreground = style.pick(&["text"]).unwrap_or(foreground);
    let muted_foreground = style
      .pick(&["text.muted", "text.placeholder"])
      .unwrap_or(foreground.with_alpha(180));
    let primary = style.pick(&["text.accent"]).unwrap_or(if dark {
      Rgba::rgb(14, 99, 156)
    } else {
      Rgba::rgb(0, 122, 204)
    });
    let primary_foreground = if primary.is_dark() {
      Rgba::rgb(255, 255, 255)
    } else {
      Rgba::rgb(20, 20, 20)
    };
    let primary_hover = primary.mix(foreground, 0.15);
    let input = style.pick(&["editor.background", "background"]).unwrap_or(background);
    let surface = style
      .pick(&["element.background", "ghost_element.background", "surface.background"])
      .unwrap_or(input);
    let list_hover = style.pick(&["element.hover", "ghost_element.hover"]).unwrap_or(hover);
    let list_active = style
      .pick(&["element.selected", "ghost_element.selected"])
      .unwrap_or(primary.with_alpha(120));
    let caret = style
      .cursor()
      .or_else(|| style.pick(&["editor.foreground", "text"]))
      .unwrap_or(foreground);
    let git_added = style
      .pick(&["version_control.added", "created"])
      .unwrap_or(Rgba::rgb(129, 184, 139));
    let git_modified = style
      .pick(&["version_control.modified", "modified"])
      .unwrap_or(Rgba::rgb(226, 192, 141));
    let git_deleted = style
      .pick(&["version_control.deleted", "deleted"])
      .unwrap_or(Rgba::rgb(200, 116, 112));
    let terminal_foreground = style.pick(&["terminal.foreground", "text"]).unwrap_or(foreground);
    Self {
      kind: spec.kind,
      background,
      foreground,
      sidebar,
      sidebar_foreground,
      sidebar_border: style.pick(&["border.variant", "border"]).unwrap_or(border),
      title_bar: style
        .pick(&["title_bar.background", "surface.background"])
        .unwrap_or(sidebar),
      title_bar_foreground: sidebar_foreground,
      status_bar: style
        .pick(&["status_bar.background", "surface.background"])
        .unwrap_or(sidebar),
      status_bar_foreground: muted_foreground,
      border,
      primary,
      primary_foreground,
      primary_hover,
      secondary: surface,
      secondary_foreground: foreground,
      secondary_hover: style
        .pick(&["element.hover", "ghost_element.hover"])
        .unwrap_or(surface.mix(foreground, 0.1)),
      muted: surface,
      muted_foreground,
      accent: list_hover,
      input,
      input_border: style.pick(&["border", "border.variant"]).unwrap_or(border),
      ring: style.pick(&["border.focused", "border.selected"]).unwrap_or(primary),
      caret,
      list_active,
      list_active_foreground: foreground,
      list_hover,
      list_inactive: style
        .pick(&["element.active", "element.background"])
        .unwrap_or(list_active.with_alpha(90)),
      popover: style
        .pick(&["elevated_surface.background", "surface.background"])
        .unwrap_or(sidebar),
      popover_foreground: foreground,
      popover_border: style.pick(&["border.variant", "border"]).unwrap_or(border),
      selection: style
        .selection()
        .or_else(|| style.pick(&["element.selected"]))
        .unwrap_or(primary.with_alpha(90)),
      link: style.pick(&["text.accent"]).unwrap_or(primary),
      link_hover: style.pick(&["link_text.hover", "text.accent"]).unwrap_or(primary_hover),
      danger: style.pick(&["error"]).unwrap_or(Rgba::rgb(241, 76, 76)),
      warning: style.pick(&["warning"]).unwrap_or(Rgba::rgb(204, 167, 0)),
      success: style
        .pick(&["success", "created", "terminal.ansi.green"])
        .unwrap_or(Rgba::rgb(137, 209, 133)),
      info: style
        .pick(&["info", "text.accent", "terminal.ansi.blue"])
        .unwrap_or(Rgba::rgb(55, 148, 255)),
      scrollbar_thumb: style
        .pick(&["scrollbar.thumb.background"])
        .unwrap_or(foreground.with_alpha(60)),
      scrollbar_thumb_hover: style
        .pick(&["scrollbar.thumb.hover_background", "scrollbar.thumb.background"])
        .unwrap_or(foreground.with_alpha(100)),
      badge: style
        .pick(&["element.selected", "element.background"])
        .unwrap_or(primary),
      badge_foreground: foreground,
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
      git_renamed: style.pick(&["version_control.renamed", "renamed"]).unwrap_or(git_added),
      git_untracked: git_added,
      git_ignored: style
        .pick(&["ignored", "text.disabled", "text.muted"])
        .unwrap_or(muted_foreground),
      git_conflicting: style
        .pick(&["version_control.conflict", "conflict"])
        .unwrap_or(Rgba::rgb(229, 148, 0)),
      git_staged_modified: git_modified,
      git_staged_deleted: git_deleted,
      diff_inserted_line: style.pick(&["created.background"]).unwrap_or(git_added.with_alpha(38)),
      diff_removed_line: style
        .pick(&["deleted.background"])
        .unwrap_or(git_deleted.with_alpha(38)),
      diff_inserted_text: style
        .pick(&["version_control.word_added"])
        .unwrap_or(git_added.with_alpha(90)),
      diff_removed_text: style
        .pick(&["version_control.word_deleted"])
        .unwrap_or(git_deleted.with_alpha(90)),
      gutter_added: git_added,
      gutter_modified: git_modified,
      gutter_deleted: git_deleted,
      terminal_background: style
        .pick(&["terminal.background", "editor.background", "background"])
        .unwrap_or(background),
      terminal_foreground,
      terminal_cursor: style
        .cursor()
        .or_else(|| style.pick(&["terminal.foreground"]))
        .unwrap_or(caret),
      terminal_ansi: ansi_palette(&style, dark),
    }
  }
}

/// The 16 ANSI slots, each with the key that stands in when the theme skips it.
const ANSI_KEYS: [(&str, &str); 16] = [
  ("terminal.ansi.black", "terminal.ansi.dim_black"),
  ("terminal.ansi.red", "terminal.ansi.dim_red"),
  ("terminal.ansi.green", "terminal.ansi.dim_green"),
  ("terminal.ansi.yellow", "terminal.ansi.dim_yellow"),
  ("terminal.ansi.blue", "terminal.ansi.dim_blue"),
  ("terminal.ansi.magenta", "terminal.ansi.dim_magenta"),
  ("terminal.ansi.cyan", "terminal.ansi.dim_cyan"),
  ("terminal.ansi.white", "terminal.ansi.dim_white"),
  ("terminal.ansi.bright_black", "terminal.ansi.black"),
  ("terminal.ansi.bright_red", "terminal.ansi.red"),
  ("terminal.ansi.bright_green", "terminal.ansi.green"),
  ("terminal.ansi.bright_yellow", "terminal.ansi.yellow"),
  ("terminal.ansi.bright_blue", "terminal.ansi.blue"),
  ("terminal.ansi.bright_magenta", "terminal.ansi.magenta"),
  ("terminal.ansi.bright_cyan", "terminal.ansi.cyan"),
  ("terminal.ansi.bright_white", "terminal.ansi.white"),
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

fn ansi_palette(style: &Resolver<'_>, dark: bool) -> [Rgba; 16] {
  let mut colors = if dark { ANSI_DARK } else { ANSI_LIGHT };
  for (slot, (key, alias)) in ANSI_KEYS.iter().enumerate() {
    if let Some(color) = style.pick(&[key, alias]) {
      colors[slot] = color;
    }
  }
  colors
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::theme::parse_theme_family;
  use core::prelude::v1::test;

  const ONE: &str = include_str!("../../../../assets/themes/one.json");
  const WARM_BURNOUT: &str = include_str!("../../../../assets/themes/warm-burnout.json");
  const AYU: &str = include_str!("../../../../assets/themes/ayu.json");
  const GRUVBOX: &str = include_str!("../../../../assets/themes/gruvbox.json");

  fn theme(json: &str, name: &str) -> ThemeSpec {
    parse_theme_family(json)
      .unwrap()
      .themes
      .into_iter()
      .find(|theme| theme.name == name)
      .unwrap_or_else(|| panic!("{name} is bundled"))
  }

  fn base(kind: ThemeKind) -> ThemeStyle {
    parse_theme_family(ONE)
      .unwrap()
      .themes
      .into_iter()
      .find(|theme| theme.kind == kind)
      .map(|theme| theme.style)
      .unwrap()
  }

  fn bundled() -> Vec<ThemeSpec> {
    [ONE, WARM_BURNOUT, AYU, GRUVBOX]
      .iter()
      .flat_map(|json| parse_theme_family(json).unwrap().themes)
      .collect()
  }

  #[test]
  fn roles_come_from_the_theme_that_declares_them() {
    let spec = theme(WARM_BURNOUT, "Warm Burnout Dark");
    let palette = UiPalette::resolve(&spec, &base(ThemeKind::Dark));
    let style = &spec.style;
    assert_eq!(Some(palette.background), style.color("editor.background"));
    assert_eq!(Some(palette.sidebar), style.color("panel.background"));
    assert_eq!(Some(palette.status_bar), style.color("status_bar.background"));
    assert_eq!(Some(palette.border), style.color("border"));
    assert_eq!(Some(palette.list_hover), style.color("element.hover"));
    assert_eq!(Some(palette.ring), style.color("border.focused"));
    assert_eq!(Some(palette.caret), style.cursor());
    assert_eq!(Some(palette.selection), style.selection());
    assert_eq!(Some(palette.danger), style.color("error"));
  }

  #[test]
  fn an_absent_key_takes_a_sibling_before_the_base_theme() {
    // Warm Burnout declares `created` but not `version_control.added`.
    let spec = theme(WARM_BURNOUT, "Warm Burnout Dark");
    let one = base(ThemeKind::Dark);
    assert!(spec.style.color("version_control.added").is_none());
    let palette = UiPalette::resolve(&spec, &one);
    assert_eq!(Some(palette.git_added), spec.style.color("created"));
    assert_ne!(Some(palette.git_added), one.color("version_control.added"));
    assert_eq!(Some(palette.git_modified), spec.style.color("modified"));
    assert_eq!(Some(palette.git_deleted), spec.style.color("deleted"));
  }

  #[test]
  fn the_base_theme_fills_what_the_theme_and_its_siblings_lack() {
    let spec = ThemeSpec {
      name: "Bare".into(),
      kind: ThemeKind::Dark,
      style: ThemeStyle::default(),
    };
    let one = base(ThemeKind::Dark);
    let palette = UiPalette::resolve(&spec, &one);
    assert_eq!(Some(palette.background), one.color("editor.background"));
    assert_eq!(Some(palette.git_added), one.color("version_control.added"));
    assert_eq!(Some(palette.scrollbar_thumb), one.color("scrollbar.thumb.background"));
    assert_eq!(palette.kind, ThemeKind::Dark);
  }

  #[test]
  fn bright_ansi_slots_fall_back_to_their_normal_slot() {
    let json = r##"{"name":"T","themes":[{"name":"T","appearance":"dark","style":{"terminal.ansi.red":"#ff0000"}}]}"##;
    let spec = parse_theme_family(json).unwrap().themes.pop().unwrap();
    let palette = UiPalette::resolve(&spec, &ThemeStyle::default());
    assert_eq!(palette.terminal_ansi[1], Rgba::rgb(0xff, 0, 0));
    assert_eq!(palette.terminal_ansi[9], Rgba::rgb(0xff, 0, 0));
    assert_eq!(palette.terminal_ansi[0], ANSI_DARK[0]);
  }

  #[test]
  fn every_bundled_theme_resolves_its_own_chrome_and_git_colors() {
    for spec in bundled() {
      let palette = UiPalette::resolve(&spec, &base(spec.kind));
      let style = &spec.style;
      assert_eq!(
        Some(palette.background),
        style.color("editor.background").or_else(|| style.color("background")),
        "{} background",
        spec.name
      );
      for (role, keys) in [
        (palette.git_added, ["version_control.added", "created"]),
        (palette.git_modified, ["version_control.modified", "modified"]),
        (palette.git_deleted, ["version_control.deleted", "deleted"]),
      ] {
        let own = keys.iter().find_map(|key| style.color(key));
        assert_eq!(Some(role), own, "{} {keys:?}", spec.name);
      }
      assert_eq!(palette.kind, spec.kind);
      assert_ne!(palette.terminal_ansi[1], ANSI_DARK[1], "{} ansi red", spec.name);
    }
  }

  #[test]
  fn light_themes_carry_the_light_mark_and_ansi_defaults() {
    let spec = theme(WARM_BURNOUT, "Warm Burnout Light");
    let palette = UiPalette::resolve(&spec, &base(ThemeKind::Light));
    assert_eq!(palette.mark, Rgba::rgb(0, 0, 0));
    assert_eq!(palette.kind, ThemeKind::Light);
    assert!(!palette.background.is_dark());
  }
}
