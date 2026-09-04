pub fn default_name(n: usize) -> String {
  format!("Terminal {n}")
}

pub fn display_name(default: &str, shell: Option<&str>, foreground: Option<&str>) -> String {
  if let Some(name) = foreground.filter(|name| !name.is_empty()) {
    return name.to_string();
  }
  if let Some(name) = shell.filter(|name| !name.is_empty()) {
    return name.to_string();
  }
  default.to_string()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_and_display_names() {
    assert_eq!(default_name(1), "Terminal 1");
    assert_eq!(default_name(12), "Terminal 12");
    assert_eq!(display_name("Terminal 1", None, None), "Terminal 1");
    assert_eq!(display_name("Terminal 1", Some("zsh"), None), "zsh");
    assert_eq!(display_name("Terminal 1", Some("zsh"), Some("cargo")), "cargo");
    assert_eq!(display_name("Terminal 1", None, Some("node")), "node");
    assert_eq!(display_name("Terminal 1", Some(""), Some("")), "Terminal 1");
  }
}
