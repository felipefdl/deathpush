//! tm-themes `tokenColors` scopes mapped to tree-sitter capture names for the diff and file viewers.

use super::{Rgba, ThemeSpec};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntaxStyle {
  pub capture: &'static str,
  pub color: Option<Rgba>,
  pub italic: bool,
  pub bold: bool,
}

/// (capture, TextMate scope prefixes in priority order). The first token color whose scope starts with
/// one of the prefixes wins; a scope prefix earlier in the list beats a later one.
const CAPTURES: &[(&str, &[&str])] = &[
  ("attribute", &["entity.other.attribute-name", "meta.attribute"]),
  ("boolean", &["constant.language.boolean", "constant.language"]),
  ("comment", &["comment"]),
  (
    "comment.doc",
    &["comment.block.documentation", "comment.line.documentation", "comment"],
  ),
  ("constant", &["constant.language", "constant.other", "constant"]),
  (
    "constructor",
    &[
      "entity.name.function.constructor",
      "entity.name.type",
      "entity.name.function",
    ],
  ),
  ("embedded", &["meta.embedded", "source"]),
  ("emphasis", &["markup.italic"]),
  ("emphasis.strong", &["markup.bold"]),
  ("enum", &["entity.name.type.enum", "entity.name.type"]),
  ("function", &["entity.name.function", "support.function"]),
  (
    "keyword",
    &["keyword.control", "keyword", "storage.type", "storage.modifier"],
  ),
  ("label", &["entity.name.label"]),
  ("link_text", &["string.other.link", "markup.underline.link"]),
  ("link_uri", &["markup.underline.link"]),
  ("number", &["constant.numeric"]),
  ("operator", &["keyword.operator"]),
  ("preproc", &["meta.preprocessor", "keyword.control.directive"]),
  (
    "property",
    &[
      "variable.other.property",
      "support.type.property-name",
      "variable.other.member",
    ],
  ),
  ("punctuation", &["punctuation"]),
  (
    "punctuation.bracket",
    &["punctuation.section", "punctuation.definition.bracket", "punctuation"],
  ),
  (
    "punctuation.delimiter",
    &["punctuation.separator", "punctuation.terminator", "punctuation"],
  ),
  (
    "punctuation.special",
    &[
      "punctuation.definition.template-expression",
      "punctuation.section.embedded",
      "punctuation",
    ],
  ),
  ("string", &["string"]),
  ("string.escape", &["constant.character.escape"]),
  ("string.regex", &["string.regexp"]),
  ("string.special", &["string.other", "string"]),
  ("string.special.symbol", &["constant.other.symbol", "string"]),
  ("tag", &["entity.name.tag"]),
  ("tag.doctype", &["meta.tag.sgml.doctype", "entity.name.tag"]),
  ("text.literal", &["markup.raw", "markup.inline.raw"]),
  ("title", &["markup.heading", "entity.name.section"]),
  ("type", &["entity.name.type", "support.type", "storage.type"]),
  ("variable", &["variable.other", "variable"]),
  ("variable.special", &["variable.language", "variable.parameter"]),
  (
    "variant",
    &[
      "entity.name.type.variant",
      "variable.other.enummember",
      "constant.other.enum",
    ],
  ),
];

fn style_for_prefix(spec: &ThemeSpec, prefix: &str) -> Option<SyntaxStyle> {
  spec.token_colors.iter().find_map(|token| {
    let scope = token.scope.as_ref()?;
    let matches = scope
      .iter()
      .any(|s| s == prefix || s.starts_with(&format!("{prefix}.")));
    if !matches {
      return None;
    }
    let settings = &token.settings;
    let color = settings.foreground.as_deref().and_then(Rgba::parse);
    let font_style = settings.font_style.clone().unwrap_or_default();
    if color.is_none() && font_style.is_empty() {
      return None;
    }
    Some(SyntaxStyle {
      capture: "",
      color,
      italic: font_style.contains("italic"),
      bold: font_style.contains("bold"),
    })
  })
}

pub fn syntax_styles(spec: &ThemeSpec) -> Vec<SyntaxStyle> {
  CAPTURES
    .iter()
    .filter_map(|(capture, prefixes)| {
      prefixes
        .iter()
        .find_map(|prefix| style_for_prefix(spec, prefix))
        .map(|style| SyntaxStyle { capture, ..style })
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::theme::parse_theme;

  #[test]
  fn maps_scopes_to_captures_by_prefix_priority() {
    let spec = parse_theme(
      r##"{"name":"t","type":"dark","colors":{},"tokenColors":[
        {"scope":"comment","settings":{"foreground":"#6a9955","fontStyle":"italic"}},
        {"scope":["keyword.control","storage.type"],"settings":{"foreground":"#c586c0"}},
        {"scope":"keyword","settings":{"foreground":"#569cd6"}},
        {"scope":"string.regexp","settings":{"foreground":"#d16969"}},
        {"scope":"entity.name.function","settings":{"foreground":"#dcdcaa","fontStyle":"bold"}}
      ]}"##,
    )
    .unwrap();
    let styles = syntax_styles(&spec);
    let get = |name: &str| styles.iter().find(|s| s.capture == name).cloned();
    let comment = get("comment").unwrap();
    assert_eq!(comment.color, Rgba::parse("#6a9955"));
    assert!(comment.italic && !comment.bold);
    assert_eq!(get("keyword").unwrap().color, Rgba::parse("#c586c0"));
    assert_eq!(get("string.regex").unwrap().color, Rgba::parse("#d16969"));
    assert!(get("function").unwrap().bold);
    assert!(get("number").is_none());
  }

  #[test]
  fn scope_match_is_prefix_on_dot_boundaries() {
    let spec = parse_theme(
      r##"{"name":"t","type":"dark","colors":{},"tokenColors":[
        {"scope":"stringy","settings":{"foreground":"#ff0000"}},
        {"scope":"string.quoted","settings":{"foreground":"#00ff00"}}
      ]}"##,
    )
    .unwrap();
    let styles = syntax_styles(&spec);
    let string = styles.iter().find(|s| s.capture == "string").unwrap();
    assert_eq!(string.color, Rgba::parse("#00ff00"));
  }
}
