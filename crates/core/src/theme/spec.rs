use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
  Dark,
  Light,
}

impl ThemeKind {
  pub fn is_dark(self) -> bool {
    self == Self::Dark
  }
}

/// One `syntax` entry. The key is already a tree-sitter capture name.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct SyntaxToken {
  pub color: Option<String>,
  pub font_style: Option<String>,
  pub font_weight: Option<u16>,
}

/// One cursor slot. Zed authors the caret and the selection wash here.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct Player {
  pub cursor: Option<String>,
  pub background: Option<String>,
  pub selection: Option<String>,
}

/// A theme's `style`: the flat color keys, plus syntax and players.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ThemeStyle {
  #[serde(default)]
  pub syntax: BTreeMap<String, SyntaxToken>,
  #[serde(default)]
  pub players: Vec<Player>,
  /// Every other `style` key. Values are usually hex strings, sometimes null.
  #[serde(flatten)]
  pub colors: BTreeMap<String, Value>,
}

impl ThemeStyle {
  /// The color at `key`, when the theme declares one and it parses.
  pub fn color(&self, key: &str) -> Option<Rgba> {
    Rgba::parse(self.colors.get(key)?.as_str()?)
  }

  /// The first cursor slot's caret color.
  pub fn cursor(&self) -> Option<Rgba> {
    Rgba::parse(self.players.first()?.cursor.as_deref()?)
  }

  /// The first cursor slot's selection wash.
  pub fn selection(&self) -> Option<Rgba> {
    Rgba::parse(self.players.first()?.selection.as_deref()?)
  }
}

/// One theme inside a family.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeSpec {
  pub name: String,
  #[serde(rename = "appearance")]
  pub kind: ThemeKind,
  #[serde(default)]
  pub style: ThemeStyle,
}

impl ThemeSpec {
  /// The catalog id: the name lowercased, with runs of other characters collapsed to `-`.
  pub fn id(&self) -> String {
    let mut id = String::with_capacity(self.name.len());
    for ch in self.name.chars() {
      if ch.is_ascii_alphanumeric() {
        id.push(ch.to_ascii_lowercase());
      } else if !id.ends_with('-') {
        id.push('-');
      }
    }
    let end = id.trim_end_matches('-').len();
    id.truncate(end);
    id
  }

  /// The name as authored, for the picker and the settings row.
  pub fn label(&self) -> String {
    self.name.clone()
  }
}

/// One theme file: several themes sharing a name and an author.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeFamily {
  pub name: String,
  #[serde(default)]
  pub author: Option<String>,
  pub themes: Vec<ThemeSpec>,
}

/// Parse a Zed theme family. Empty families are rejected: they would register nothing.
pub fn parse_theme_family(json: &str) -> Result<ThemeFamily> {
  let family: ThemeFamily = serde_json::from_str(json).map_err(|err| Error::Other(format!("theme parse: {err}")))?;
  if family.themes.is_empty() {
    return Err(Error::Other(format!("theme parse: {} declares no themes", family.name)));
  }
  Ok(family)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rgba {
  pub r: u8,
  pub g: u8,
  pub b: u8,
  pub a: u8,
}

impl Rgba {
  pub const fn rgb(r: u8, g: u8, b: u8) -> Self {
    Self { r, g, b, a: 255 }
  }

  /// `#rgb`, `#rgba`, `#rrggbb`, or `#rrggbbaa`.
  pub fn parse(text: &str) -> Option<Self> {
    let hex = text.trim().strip_prefix('#')?;
    let expand = |c: char| u8::from_str_radix(&format!("{c}{c}"), 16).ok();
    let pair = |s: &str| u8::from_str_radix(s, 16).ok();
    let chars: Vec<char> = hex.chars().collect();
    match chars.len() {
      3 => Some(Self {
        r: expand(chars[0])?,
        g: expand(chars[1])?,
        b: expand(chars[2])?,
        a: 255,
      }),
      4 => Some(Self {
        r: expand(chars[0])?,
        g: expand(chars[1])?,
        b: expand(chars[2])?,
        a: expand(chars[3])?,
      }),
      6 => Some(Self {
        r: pair(&hex[0..2])?,
        g: pair(&hex[2..4])?,
        b: pair(&hex[4..6])?,
        a: 255,
      }),
      8 => Some(Self {
        r: pair(&hex[0..2])?,
        g: pair(&hex[2..4])?,
        b: pair(&hex[4..6])?,
        a: pair(&hex[6..8])?,
      }),
      _ => None,
    }
  }

  pub fn to_hex(self) -> String {
    if self.a == 255 {
      format!("#{:02x}{:02x}{:02x}", self.r, self.g, self.b)
    } else {
      format!("#{:02x}{:02x}{:02x}{:02x}", self.r, self.g, self.b, self.a)
    }
  }

  pub fn with_alpha(self, a: u8) -> Self {
    Self { a, ..self }
  }

  /// Linear blend toward `other` by `t` in 0..=1.
  pub fn mix(self, other: Rgba, t: f32) -> Self {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: u8, b: u8| (a as f32 + (b as f32 - a as f32) * t).round() as u8;
    Self {
      r: lerp(self.r, other.r),
      g: lerp(self.g, other.g),
      b: lerp(self.b, other.b),
      a: lerp(self.a, other.a),
    }
  }

  pub fn is_dark(self) -> bool {
    let luma = 0.2126 * self.r as f32 + 0.7152 * self.g as f32 + 0.0722 * self.b as f32;
    luma < 128.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  const FAMILY: &str = r##"{
    "name": "Test Family",
    "author": "Nobody",
    "themes": [
      {
        "name": "Test Dark",
        "appearance": "dark",
        "style": {
          "background": "#101010",
          "text": "#eeeeee",
          "border.transparent": "#00000000",
          "missing": null,
          "players": [{ "cursor": "#ff0000", "selection": "#ff000033" }],
          "syntax": { "keyword": { "color": "#b477cf", "font_style": "italic", "font_weight": null } }
        }
      },
      { "name": "Test Light", "appearance": "light", "style": {} }
    ]
  }"##;

  #[test]
  fn parses_every_theme_in_a_family() {
    let family = parse_theme_family(FAMILY).unwrap();
    assert_eq!(family.name, "Test Family");
    assert_eq!(family.author.as_deref(), Some("Nobody"));
    let kinds: Vec<ThemeKind> = family.themes.iter().map(|theme| theme.kind).collect();
    assert_eq!(kinds, vec![ThemeKind::Dark, ThemeKind::Light]);
  }

  #[test]
  fn style_reads_colors_players_and_syntax() {
    let family = parse_theme_family(FAMILY).unwrap();
    let style = &family.themes[0].style;
    assert_eq!(style.color("background"), Some(Rgba::rgb(0x10, 0x10, 0x10)));
    assert_eq!(
      style.color("border.transparent"),
      Some(Rgba::rgb(0, 0, 0).with_alpha(0))
    );
    assert_eq!(style.color("missing"), None);
    assert_eq!(style.color("absent"), None);
    assert_eq!(style.cursor(), Some(Rgba::rgb(0xff, 0, 0)));
    assert_eq!(style.selection(), Some(Rgba::rgb(0xff, 0, 0).with_alpha(0x33)));
    assert_eq!(style.syntax["keyword"].font_style.as_deref(), Some("italic"));
    assert!(!style.colors.contains_key("syntax"));
    assert!(!style.colors.contains_key("players"));
  }

  #[test]
  fn ids_are_slugs_of_the_name() {
    let family = parse_theme_family(FAMILY).unwrap();
    assert_eq!(family.themes[0].id(), "test-dark");
    assert_eq!(family.themes[0].label(), "Test Dark");
    let odd = ThemeSpec {
      name: "Rosé Pine (Moon)!".into(),
      kind: ThemeKind::Dark,
      style: ThemeStyle::default(),
    };
    assert_eq!(odd.id(), "ros-pine-moon");
  }

  #[test]
  fn a_family_without_themes_is_rejected() {
    let err = parse_theme_family(r#"{"name":"Empty","themes":[]}"#).unwrap_err();
    assert!(err.to_string().contains("declares no themes"));
  }

  #[test]
  fn hex_parses_every_length() {
    assert_eq!(Rgba::parse("#fff"), Some(Rgba::rgb(255, 255, 255)));
    assert_eq!(Rgba::parse("#0f08"), Some(Rgba::rgb(0, 255, 0).with_alpha(0x88)));
    assert_eq!(Rgba::parse("#74ade8"), Some(Rgba::rgb(0x74, 0xad, 0xe8)));
    assert_eq!(
      Rgba::parse("#74ade83d"),
      Some(Rgba::rgb(0x74, 0xad, 0xe8).with_alpha(0x3d))
    );
    assert_eq!(Rgba::parse("74ade8"), None);
    assert_eq!(Rgba::parse("#12345"), None);
  }

  #[test]
  fn hex_round_trips_and_mixes() {
    assert_eq!(Rgba::rgb(0, 0, 0).to_hex(), "#000000");
    assert_eq!(Rgba::rgb(0, 0, 0).with_alpha(0x80).to_hex(), "#00000080");
    assert_eq!(
      Rgba::rgb(0, 0, 0).mix(Rgba::rgb(255, 255, 255), 0.5),
      Rgba::rgb(128, 128, 128)
    );
    assert!(Rgba::rgb(20, 20, 20).is_dark());
    assert!(!Rgba::rgb(240, 240, 240).is_dark());
  }
}
