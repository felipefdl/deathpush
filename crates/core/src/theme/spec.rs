use std::collections::BTreeMap;

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeKind {
  Dark,
  Light,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum Scope {
  One(String),
  Many(Vec<String>),
}

impl Scope {
  pub fn iter(&self) -> Box<dyn Iterator<Item = &str> + '_> {
    match self {
      Scope::One(scope) => Box::new(scope.split(',').map(str::trim)),
      Scope::Many(scopes) => Box::new(scopes.iter().map(String::as_str)),
    }
  }
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TokenSettings {
  pub foreground: Option<String>,
  pub background: Option<String>,
  #[serde(rename = "fontStyle")]
  pub font_style: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TokenColor {
  pub name: Option<String>,
  pub scope: Option<Scope>,
  #[serde(default)]
  pub settings: TokenSettings,
}

/// One VS Code style theme as shipped by tm-themes.
#[derive(Debug, Clone, Deserialize)]
pub struct ThemeSpec {
  pub name: String,
  #[serde(rename = "displayName")]
  pub display_name: Option<String>,
  #[serde(rename = "type")]
  pub kind: ThemeKind,
  #[serde(default, deserialize_with = "deserialize_colors")]
  pub colors: BTreeMap<String, String>,
  #[serde(default, rename = "tokenColors")]
  pub token_colors: Vec<TokenColor>,
}

fn deserialize_colors<'de, D>(deserializer: D) -> std::result::Result<BTreeMap<String, String>, D::Error>
where
  D: serde::Deserializer<'de>,
{
  let raw = BTreeMap::<String, serde_json::Value>::deserialize(deserializer)?;
  Ok(
    raw
      .into_iter()
      .filter_map(|(key, value)| match value {
        serde_json::Value::String(text) => Some((key, text)),
        _ => None,
      })
      .collect(),
  )
}

pub fn parse_theme(json: &str) -> Result<ThemeSpec> {
  serde_json::from_str(json).map_err(|err| Error::Other(format!("theme parse: {err}")))
}

impl ThemeSpec {
  /// The display name, or the id title-cased on hyphens.
  pub fn label(&self) -> String {
    self.display_name.clone().unwrap_or_else(|| {
      self
        .name
        .split('-')
        .map(|part| {
          let mut chars = part.chars();
          match chars.next() {
            Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            None => String::new(),
          }
        })
        .collect::<Vec<_>>()
        .join(" ")
    })
  }

  pub fn color(&self, key: &str) -> Option<Rgba> {
    self.colors.get(key).and_then(|value| Rgba::parse(value))
  }
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

  const VESPER: &str = include_str!("../../../../assets/themes/vesper.json");
  const AYU_LIGHT: &str = include_str!("../../../../assets/themes/ayu-light.json");

  #[test]
  fn parses_the_default_themes() {
    let vesper = parse_theme(VESPER).unwrap();
    assert_eq!(vesper.name, "vesper");
    assert_eq!(vesper.kind, ThemeKind::Dark);
    assert!(vesper.color("editor.background").is_some());
    assert!(!vesper.token_colors.is_empty());
    let ayu = parse_theme(AYU_LIGHT).unwrap();
    assert_eq!(ayu.kind, ThemeKind::Light);
  }

  #[test]
  fn label_uses_display_name_or_title_case() {
    let vesper = parse_theme(VESPER).unwrap();
    assert_eq!(vesper.label(), vesper.display_name.clone().unwrap());
    let spec = ThemeSpec {
      name: "one-dark-pro".into(),
      display_name: None,
      kind: ThemeKind::Dark,
      colors: BTreeMap::new(),
      token_colors: vec![],
    };
    assert_eq!(spec.label(), "One Dark Pro");
  }

  #[test]
  fn rgba_parses_every_hex_form() {
    assert_eq!(Rgba::parse("#fff"), Some(Rgba::rgb(255, 255, 255)));
    assert_eq!(Rgba::parse("#1e1e1e"), Some(Rgba::rgb(30, 30, 30)));
    assert_eq!(
      Rgba::parse("#1e1e1e80"),
      Some(Rgba {
        r: 30,
        g: 30,
        b: 30,
        a: 128
      })
    );
    assert_eq!(
      Rgba::parse("#abcd"),
      Some(Rgba {
        r: 170,
        g: 187,
        b: 204,
        a: 221
      })
    );
    assert_eq!(Rgba::parse("red"), None);
    assert_eq!(Rgba::rgb(30, 30, 30).to_hex(), "#1e1e1e");
    assert_eq!(Rgba { r: 1, g: 2, b: 3, a: 4 }.to_hex(), "#01020304");
  }

  #[test]
  fn scope_iterates_strings_and_arrays() {
    let one = Scope::One("comment, string".into());
    assert_eq!(one.iter().collect::<Vec<_>>(), vec!["comment", "string"]);
    let many = Scope::Many(vec!["keyword".into()]);
    assert_eq!(many.iter().collect::<Vec<_>>(), vec!["keyword"]);
  }

  #[test]
  fn every_bundled_theme_parses() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../assets/themes");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
      let path = entry.unwrap().path();
      if path.extension().is_some_and(|ext| ext == "json") {
        let json = std::fs::read_to_string(&path).unwrap();
        parse_theme(&json).unwrap_or_else(|err| panic!("{}: {err}", path.display()));
        count += 1;
      }
    }
    assert_eq!(count, 65);
  }
}
