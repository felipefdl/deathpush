//! Core's `detect_language` names mapped to gpui-component grammar names.

/// Returns the gpui-component language name for a core language, or `None` when no grammar is compiled in.
#[allow(dead_code)]
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn maps_core_names_to_grammars() {
    assert_eq!(grammar_name("typescript"), Some("typescript"));
    assert_eq!(grammar_name("shell"), Some("bash"));
    assert_eq!(grammar_name("scss"), Some("css"));
    assert_eq!(grammar_name("ini"), None);
    assert_eq!(grammar_name("dotenv"), None);
  }
}
