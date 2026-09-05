use std::path::Path;

use super::settings::{WorkspaceEntry, ZOOM_MAX, ZOOM_MIN, zoom_scale};

const UNIX_SHELLS: &[&str] = &[
  "/bin/zsh",
  "/bin/bash",
  "/usr/bin/fish",
  "/opt/homebrew/bin/fish",
  "/bin/sh",
];
const WINDOWS_SHELLS: &[&str] = &["powershell.exe", "pwsh.exe", "cmd.exe"];

/// `dir` or `dir:depth` (depth above 1), joined by `, `; empty → None (the UI shows `Not configured`).
pub fn workspace_summary(entries: &[WorkspaceEntry]) -> Option<String> {
  if entries.is_empty() {
    return None;
  }
  Some(
    entries
      .iter()
      .map(|entry| {
        if entry.scan_depth > 1 {
          format!("{}:{}", entry.directory, entry.scan_depth)
        } else {
          entry.directory.clone()
        }
      })
      .collect::<Vec<_>>()
      .join(", "),
  )
}

/// (level, label) for levels -5..=9, label `{round(1.2^level*100)}%`.
pub fn zoom_options() -> Vec<(i32, String)> {
  (ZOOM_MIN..=ZOOM_MAX)
    .map(|level| {
      let percent = (zoom_scale(level) * 100.0).round() as i32;
      (level, format!("{percent}%"))
    })
    .collect()
}

/// A shell path choice for the Settings Shell Path control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellPreset {
  /// Use `$SHELL` (empty stored path).
  Default,
  /// A platform shell that exists on disk.
  Path(String),
  /// A user-entered path not in the platform list.
  Custom,
}

impl ShellPreset {
  /// Select label: `Default ($SHELL)`, the path, or `Custom...`.
  pub fn label(&self) -> String {
    match self {
      Self::Default => "Default ($SHELL)".to_string(),
      Self::Path(path) => path.clone(),
      Self::Custom => "Custom...".to_string(),
    }
  }
}

/// Whether a shell path exists. Absolute and relative paths check the filesystem. Bare names search `env_path` (PATH), appending each `path_ext` suffix (PATHEXT) when given.
pub fn shell_exists(path: &str, env_path: Option<&str>, path_ext: Option<&str>) -> bool {
  let file = Path::new(path);
  if file.is_absolute() || path.contains('/') || path.contains('\\') {
    return file.exists();
  }
  let Some(env_path) = env_path else {
    return false;
  };
  let sep = if cfg!(windows) { ';' } else { ':' };
  let suffixes: Vec<&str> = path_ext.unwrap_or("").split(';').filter(|s| !s.is_empty()).collect();
  for dir in env_path.split(sep).filter(|dir| !dir.is_empty()) {
    if Path::new(dir).join(path).exists() {
      return true;
    }
    for suffix in &suffixes {
      if has_ascii_suffix(path, suffix) {
        continue;
      }
      if Path::new(dir).join(format!("{path}{suffix}")).exists() {
        return true;
      }
    }
  }
  false
}

fn has_ascii_suffix(name: &str, suffix: &str) -> bool {
  let name = name.as_bytes();
  let suffix = suffix.as_bytes();
  name.len() >= suffix.len() && name[name.len() - suffix.len()..].eq_ignore_ascii_case(suffix)
}

/// Default, then the platform shells that exist (`exists` decides, injected for tests), then Custom.
pub fn shell_presets(exists: &dyn Fn(&str) -> bool) -> Vec<ShellPreset> {
  let mut presets = vec![ShellPreset::Default];
  let shells = if cfg!(windows) { WINDOWS_SHELLS } else { UNIX_SHELLS };
  for path in shells {
    if exists(path) {
      presets.push(ShellPreset::Path((*path).to_string()));
    }
  }
  presets.push(ShellPreset::Custom);
  presets
}

/// Which preset a stored shell_path selects: empty → Default; a listed path → Path; anything else → Custom.
pub fn preset_for(shell_path: &str, presets: &[ShellPreset]) -> ShellPreset {
  if shell_path.is_empty() {
    return ShellPreset::Default;
  }
  presets
    .iter()
    .find(|preset| matches!(preset, ShellPreset::Path(path) if path == shell_path))
    .cloned()
    .unwrap_or(ShellPreset::Custom)
}

/// Font-weight options as `(display label, stored value)` pairs.
pub const FONT_WEIGHTS: [(&str, &str); 9] = [
  ("Thin", "thin"),
  ("Extra Light", "extra-light"),
  ("Light", "light"),
  ("Normal", "normal"),
  ("Medium", "medium"),
  ("Semi Bold", "semi-bold"),
  ("Bold", "bold"),
  ("Extra Bold", "extra-bold"),
  ("Black", "black"),
];
/// Confirmation dialog title for Reset to Defaults.
pub const RESET_TITLE: &str = "Reset to Defaults";
/// Confirmation dialog message for Reset to Defaults.
pub const RESET_MESSAGE: &str = "Reset all settings to defaults? This cannot be undone.";

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn workspace_summary_formats_depth_and_joins() {
    assert_eq!(workspace_summary(&[]), None);
    let entries = [
      WorkspaceEntry {
        directory: "/src".into(),
        scan_depth: 1,
      },
      WorkspaceEntry {
        directory: "/work".into(),
        scan_depth: 2,
      },
    ];
    assert_eq!(workspace_summary(&entries), Some("/src, /work:2".into()));
  }

  #[test]
  fn zoom_options_cover_the_levels() {
    let options = zoom_options();
    assert_eq!(options.len(), 15);
    assert_eq!(
      options.iter().map(|(level, _)| *level).collect::<Vec<_>>(),
      (-5..=9).collect::<Vec<_>>()
    );
    let label = |level: i32| {
      options
        .iter()
        .find(|(candidate, _)| *candidate == level)
        .map(|(_, text)| text.as_str())
        .unwrap()
    };
    assert_eq!(label(0), "100%");
    assert_eq!(label(1), "120%");
    assert_eq!(label(-1), "83%");
  }

  #[test]
  fn shell_exists_finds_bare_name_on_path() {
    let dir = tempfile::TempDir::new().unwrap();
    let exe = dir.path().join("powershell.exe");
    std::fs::write(&exe, b"").unwrap();
    std::fs::write(dir.path().join("cmd.exe"), b"").unwrap();
    let env_path = dir.path().to_str().expect("utf-8 temp path");
    assert!(shell_exists("powershell.exe", Some(env_path), Some(".EXE;.exe")));
    assert!(shell_exists("cmd", Some(env_path), Some(".EXE;.exe")));
    assert!(!shell_exists("pwsh.exe", Some(env_path), Some(".EXE;.exe")));
    assert!(!shell_exists("powershell.exe", Some("/does/not/exist"), Some(".exe")));
    assert!(shell_exists(exe.to_str().unwrap(), Some("/does/not/exist"), None));
  }

  #[test]
  fn shell_presets_filter_by_existence() {
    assert_eq!(
      shell_presets(&|_| false),
      vec![ShellPreset::Default, ShellPreset::Custom]
    );
    let keep = if cfg!(windows) { "cmd.exe" } else { "/bin/zsh" };
    assert_eq!(
      shell_presets(&|path| path == keep),
      vec![
        ShellPreset::Default,
        ShellPreset::Path(keep.to_string()),
        ShellPreset::Custom,
      ]
    );
    assert_eq!(ShellPreset::Default.label(), "Default ($SHELL)");
    assert_eq!(ShellPreset::Custom.label(), "Custom...");
    assert_eq!(ShellPreset::Path(keep.to_string()).label(), keep);
  }

  #[test]
  fn preset_for_matches_stored_path() {
    let presets = [
      ShellPreset::Default,
      ShellPreset::Path("/bin/zsh".into()),
      ShellPreset::Custom,
    ];
    assert_eq!(preset_for("", &presets), ShellPreset::Default);
    assert_eq!(preset_for("/bin/zsh", &presets), ShellPreset::Path("/bin/zsh".into()));
    assert_eq!(preset_for("/usr/local/bin/fish", &presets), ShellPreset::Custom);
  }

  #[test]
  fn font_weights_have_defaults() {
    let stored: Vec<&str> = FONT_WEIGHTS.iter().map(|(_, value)| *value).collect();
    assert!(stored.contains(&"normal"));
    assert!(stored.contains(&"bold"));
  }
}
