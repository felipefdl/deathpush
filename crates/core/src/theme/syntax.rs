//! Zed `syntax` keys, which are already tree-sitter capture names, resolved for the diff and file viewers.

use std::collections::BTreeMap;

use super::{Rgba, SyntaxToken, ThemeSpec, ThemeStyle};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxStyle {
  pub capture: String,
  pub color: Option<Rgba>,
  pub italic: bool,
  pub bold: bool,
}

/// Captures the highlighter needs, each with the sibling keys that can stand in when a theme
/// omits it. The capture itself is tried first, then these, then the same list in the base theme.
const CAPTURES: &[(&str, &[&str])] = &[
  ("attribute", &["property", "variable"]),
  ("boolean", &["constant", "number"]),
  ("comment", &[]),
  ("comment.doc", &["comment"]),
  ("constant", &["number"]),
  ("constructor", &["type", "function"]),
  ("diff.minus", &[]),
  ("diff.plus", &[]),
  ("embedded", &["variable"]),
  ("emphasis", &["variable.special"]),
  ("emphasis.strong", &["emphasis"]),
  ("enum", &["type"]),
  ("function", &[]),
  ("function.builtin", &["function"]),
  ("hint", &["comment"]),
  ("keyword", &[]),
  ("label", &["variable.special", "variable"]),
  ("link_text", &["string.special", "string"]),
  ("link_uri", &["link_text", "string"]),
  ("namespace", &["type", "variable"]),
  ("number", &["constant"]),
  ("operator", &["punctuation", "keyword"]),
  ("predictive", &["hint", "comment"]),
  ("preproc", &["keyword"]),
  ("primary", &["variable"]),
  ("property", &["variable.member", "variable"]),
  ("punctuation", &[]),
  ("punctuation.bracket", &["punctuation"]),
  ("punctuation.delimiter", &["punctuation"]),
  ("punctuation.list_marker", &["punctuation.special", "punctuation"]),
  ("punctuation.markup", &["punctuation.special", "punctuation"]),
  ("punctuation.special", &["punctuation"]),
  ("selector", &["tag", "type"]),
  ("selector.pseudo", &["selector", "attribute"]),
  ("string", &[]),
  ("string.escape", &["string.special", "string"]),
  ("string.regex", &["string.special", "string"]),
  ("string.special", &["string"]),
  ("string.special.symbol", &["string.special", "string"]),
  ("tag", &["keyword"]),
  ("text.literal", &["string"]),
  ("title", &["keyword"]),
  ("type", &[]),
  ("type.builtin", &["type"]),
  ("variable", &[]),
  ("variable.member", &["property", "variable"]),
  ("variable.parameter", &["variable"]),
  ("variable.special", &["variable"]),
  ("variant", &["enum", "type"]),
];

/// A token is worth registering only when it carries a color or a font hint.
fn resolve(capture: &str, token: &SyntaxToken) -> Option<SyntaxStyle> {
  let color = token.color.as_deref().and_then(Rgba::parse);
  let italic = matches!(token.font_style.as_deref(), Some("italic" | "oblique"));
  let bold = token.font_weight.is_some_and(|weight| weight >= 600);
  (color.is_some() || italic || bold).then(|| SyntaxStyle {
    capture: capture.to_string(),
    color,
    italic,
    bold,
  })
}

/// The first key in `keys` that the style declares with something usable.
fn first(style: &ThemeStyle, capture: &str, keys: &[&str]) -> Option<SyntaxStyle> {
  style
    .syntax
    .get(capture)
    .and_then(|token| resolve(capture, token))
    .or_else(|| {
      keys
        .iter()
        .find_map(|key| style.syntax.get(*key).and_then(|token| resolve(capture, token)))
    })
}

/// Every capture the theme declares, plus the aliased fills for the ones it omits.
pub fn syntax_styles(spec: &ThemeSpec, base: &ThemeStyle) -> Vec<SyntaxStyle> {
  let mut styles: BTreeMap<&str, SyntaxStyle> = BTreeMap::new();
  for (capture, token) in &spec.style.syntax {
    if let Some(style) = resolve(capture, token) {
      styles.insert(capture.as_str(), style);
    }
  }
  for (capture, aliases) in CAPTURES {
    if styles.contains_key(capture) {
      continue;
    }
    if let Some(style) = first(&spec.style, capture, aliases).or_else(|| first(base, capture, aliases)) {
      styles.insert(capture, style);
    }
  }
  styles.into_values().collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::theme::{ThemeKind, parse_theme_family};
  use core::prelude::v1::test;

  fn theme(syntax: &str) -> ThemeSpec {
    let json =
      format!(r##"{{"name":"T","themes":[{{"name":"T","appearance":"dark","style":{{"syntax":{syntax}}}}}]}}"##);
    parse_theme_family(&json).unwrap().themes.pop().unwrap()
  }

  fn style_of(styles: &[SyntaxStyle], capture: &str) -> Option<SyntaxStyle> {
    styles.iter().find(|style| style.capture == capture).cloned()
  }

  #[test]
  fn authored_captures_pass_straight_through() {
    let spec = theme(
      r##"{"keyword":{"color":"#b477cf","font_style":"italic"},"string":{"color":"#a1c181","font_weight":700}}"##,
    );
    let styles = syntax_styles(&spec, &ThemeStyle::default());
    let keyword = style_of(&styles, "keyword").unwrap();
    assert_eq!(keyword.color, Some(Rgba::rgb(0xb4, 0x77, 0xcf)));
    assert!(keyword.italic);
    assert!(!keyword.bold);
    assert!(style_of(&styles, "string").unwrap().bold);
  }

  #[test]
  fn a_missing_capture_takes_a_sibling_from_the_same_theme() {
    let spec = theme(r##"{"string":{"color":"#a1c181"},"comment":{"color":"#5c6370"}}"##);
    let styles = syntax_styles(&spec, &ThemeStyle::default());
    let escape = style_of(&styles, "string.escape").unwrap();
    assert_eq!(escape.color, Some(Rgba::rgb(0xa1, 0xc1, 0x81)));
    let hint = style_of(&styles, "hint").unwrap();
    assert_eq!(hint.color, Some(Rgba::rgb(0x5c, 0x63, 0x70)));
  }

  #[test]
  fn the_base_theme_fills_what_no_sibling_covers() {
    let spec = theme(r##"{"keyword":{"color":"#b477cf"}}"##);
    let base = theme(r##"{"diff.plus":{"color":"#a1c181"}}"##).style;
    let styles = syntax_styles(&spec, &base);
    assert_eq!(
      style_of(&styles, "diff.plus").unwrap().color,
      Some(Rgba::rgb(0xa1, 0xc1, 0x81))
    );
    assert!(style_of(&styles, "selector").is_none());
  }

  #[test]
  fn a_null_token_is_not_registered() {
    let spec = theme(r##"{"keyword":{"color":null,"font_style":null,"font_weight":null}}"##);
    assert!(syntax_styles(&spec, &ThemeStyle::default()).is_empty());
  }

  #[test]
  fn the_bundled_themes_resolve_their_own_captures() {
    let family = parse_theme_family(include_str!("../../../../assets/themes/warm-burnout.json")).unwrap();
    let base = parse_theme_family(include_str!("../../../../assets/themes/one.json")).unwrap();
    let base_dark = base
      .themes
      .iter()
      .find(|theme| theme.kind == ThemeKind::Dark)
      .map(|theme| theme.style.clone())
      .unwrap();
    for spec in &family.themes {
      let styles = syntax_styles(spec, &base_dark);
      let keyword = style_of(&styles, "keyword").unwrap();
      assert_eq!(
        keyword.color,
        spec.style.syntax["keyword"].color.as_deref().and_then(Rgba::parse)
      );
      // Warm Burnout omits `variant`; the alias chain fills it from its own `enum`.
      assert_eq!(
        style_of(&styles, "variant").unwrap().color,
        spec.style.syntax["enum"].color.as_deref().and_then(Rgba::parse)
      );
    }
  }
}
