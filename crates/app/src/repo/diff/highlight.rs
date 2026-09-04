//! Core's `detect_language` names mapped to gpui-component grammar names, plus a per-payload highlighter cache.

use std::ops::Range;

use deathpush_core::session::types::DiffPayload;
use gpui_kit::HighlightStyle;
use gpui_kit::base::input::HighlightStyleResolver;
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::component::input::Rope;

/// Returns the gpui-component language name for a core language, or `None` when no grammar is compiled in.
pub fn grammar_name(language: &str) -> Option<&'static str> {
  Some(match language {
    "rust" => "rust",
    "typescript" => "typescript",
    "tsx" => "tsx",
    "javascript" => "javascript",
    "json" => "json",
    "html" => "html",
    "css" | "scss" | "less" => "css",
    "markdown" | "mdx" => "markdown",
    "toml" => "toml",
    "yaml" => "yaml",
    "python" => "python",
    "go" => "go",
    "shell" => "bash",
    "sql" => "sql",
    "java" => "java",
    "kotlin" => "kotlin",
    "swift" => "swift",
    "c" => "c",
    "cpp" => "cpp",
    "csharp" => "csharp",
    "ruby" => "ruby",
    "php" => "php",
    "lua" => "lua",
    "scala" => "scala",
    "elixir" => "elixir",
    "graphql" => "graphql",
    "proto" => "proto",
    "zig" => "zig",
    "dockerfile" | "justfile" => "bash",
    _ => return None,
  })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Side {
  Old,
  New,
}

pub struct Highlighted {
  pub hash: String,
  pub line_starts_old: Vec<usize>,
  pub line_starts_new: Vec<usize>,
  old: Option<SyntaxHighlighter>,
  new: Option<SyntaxHighlighter>,
}

impl Highlighted {
  pub fn build(payload: &DiffPayload) -> Highlighted {
    let name = grammar_name(payload.language.as_deref().unwrap_or(""));
    let highlighter = |text: &str| {
      name.map(|name| {
        let mut highlighter = SyntaxHighlighter::new(name);
        highlighter.update(None, &Rope::from_str(text), None);
        highlighter
      })
    };
    let line_starts_old = line_starts(&payload.original);
    let line_starts_new = line_starts(&payload.modified);
    Highlighted {
      hash: payload.content_hash.clone(),
      line_starts_old,
      line_starts_new,
      old: highlighter(&payload.original),
      new: highlighter(&payload.modified),
    }
  }

  /// Highlight runs for one source line (0-based, `old` or `new` side), as (byte range within the line, HighlightStyle).
  pub fn line_styles(
    &self,
    side: Side,
    line: usize,
    theme: &dyn HighlightStyleResolver,
  ) -> Vec<(Range<usize>, HighlightStyle)> {
    let (highlighter, starts) = match side {
      Side::Old => (self.old.as_ref(), &self.line_starts_old),
      Side::New => (self.new.as_ref(), &self.line_starts_new),
    };
    let Some(highlighter) = highlighter else {
      return Vec::new();
    };
    let Some(&start) = starts.get(line) else {
      return Vec::new();
    };
    let text_len = highlighter.text().len();
    let end = starts
      .get(line + 1)
      .map(|next| next.saturating_sub(1))
      .unwrap_or(text_len);
    if start >= end {
      return Vec::new();
    }
    let range = start..end;
    let line_len = range.end - range.start;
    highlighter
      .styles(&range, theme)
      .into_iter()
      .filter_map(|(hit, style)| {
        let lo = hit.start.saturating_sub(range.start).min(line_len);
        let hi = hit.end.saturating_sub(range.start).min(line_len);
        (lo < hi).then_some((lo..hi, style))
      })
      .collect()
  }
}

pub fn line_starts(text: &str) -> Vec<usize> {
  let mut starts = vec![0];
  for (index, ch) in text.char_indices() {
    if ch == '\n' {
      starts.push(index + 1);
    }
  }
  starts
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::session::types::DiffPresence;
  use gpui_kit::component::highlighter::HighlightTheme;

  fn text_payload(modified: &str) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      original: String::new(),
      modified: modified.to_string(),
      language: Some("rust".into()),
      file_type: "text".into(),
      hunks: vec![],
      presence: DiffPresence {
        old_exists: true,
        new_exists: true,
      },
      editable: true,
      enable_line_selection: true,
      staged: false,
      content_hash: "h".into(),
    }
  }

  #[test]
  fn maps_core_names_to_grammars() {
    assert_eq!(grammar_name("typescript"), Some("typescript"));
    assert_eq!(grammar_name("shell"), Some("bash"));
    assert_eq!(grammar_name("scss"), Some("css"));
    assert_eq!(grammar_name("ini"), None);
    assert_eq!(grammar_name("dotenv"), None);
  }

  #[test]
  fn line_starts_are_byte_offsets() {
    assert_eq!(line_starts("ab\ncd\n\ne"), vec![0, 3, 6, 7]);
    assert_eq!(line_starts(""), vec![0]);
  }

  #[test]
  fn rust_keywords_get_a_style() {
    let payload = text_payload("fn main() {}\nlet x = 1;\n");
    let highlighted = Highlighted::build(&payload);
    let theme = HighlightTheme::default_dark();
    let styles = highlighted.line_styles(Side::New, 1, theme.as_ref());
    assert!(
      styles.iter().any(|(range, _)| range.start == 0 && range.end == 3),
      "`let` is a keyword: {styles:?}"
    );
  }
}
