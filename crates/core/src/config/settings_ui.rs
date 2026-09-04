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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellPreset {
  Default,
  Path(String),
  Custom,
}

impl ShellPreset {
  pub fn label(&self) -> String {
    match self {
      Self::Default => "Default ($SHELL)".to_string(),
      Self::Path(path) => path.clone(),
      Self::Custom => "Custom...".to_string(),
    }
  }
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
pub const RESET_TITLE: &str = "Reset to Defaults";
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
