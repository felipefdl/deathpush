use serde::{Deserialize, Serialize};

pub const ZOOM_MIN: i32 = -5;
pub const ZOOM_MAX: i32 = 9;

/// 1.2 to the power of the level. Level 3 is about 173%.
pub fn zoom_scale(level: i32) -> f32 {
  1.2f32.powi(level.clamp(ZOOM_MIN, ZOOM_MAX))
}

pub const DEFAULT_DARK_THEME: &str = "warm-burnout-dark";
pub const DEFAULT_LIGHT_THEME: &str = "warm-burnout-light";
pub const MONO_FONT_STACK: &str = "MesloLGS Nerd Font Mono";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
  pub ui: UiSettings,
  pub editor: EditorSettings,
  pub diff: DiffSettings,
  pub terminal: TerminalSettings,
  pub git: GitSettings,
  pub projects: ProjectsSettings,
  pub theme: ThemeSettings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum SidebarPosition {
  #[default]
  Left,
  Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TreeDensity {
  #[default]
  Compact,
  Default,
  Relaxed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum TreeIcons {
  Minimal,
  Standard,
  #[default]
  Complete,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UiSettings {
  pub font_family: String,
  pub font_size: u32,
  pub sidebar_position: SidebarPosition,
  pub always_open_terminal_on_start: bool,
  pub zoom_level: i32,
  pub tree_density: TreeDensity,
  pub tree_icons: TreeIcons,
}

impl Default for UiSettings {
  fn default() -> Self {
    Self {
      font_family: String::new(),
      font_size: 13,
      sidebar_position: SidebarPosition::Left,
      always_open_terminal_on_start: false,
      zoom_level: 0,
      tree_density: TreeDensity::Compact,
      tree_icons: TreeIcons::Complete,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum WordWrap {
  #[default]
  Off,
  On,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct EditorSettings {
  pub font_size: u32,
  pub font_family: String,
  pub line_height: u32,
  pub tab_size: u32,
  pub word_wrap: WordWrap,
}

impl Default for EditorSettings {
  fn default() -> Self {
    Self {
      font_size: 13,
      font_family: MONO_FONT_STACK.to_string(),
      line_height: 20,
      tab_size: 4,
      word_wrap: WordWrap::Off,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DiffLayout {
  Inline,
  #[default]
  SideBySide,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum DiffIndicators {
  Classic,
  Bars,
  #[default]
  None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum LineDiffType {
  #[default]
  WordAlt,
  Word,
  Char,
  None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum HunkSeparators {
  #[default]
  Simple,
  Metadata,
  LineInfo,
  LineInfoBasic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DiffSettings {
  pub layout: DiffLayout,
  pub show_inline_hunk_actions: bool,
  pub show_line_numbers: bool,
  pub diff_indicators: DiffIndicators,
  pub line_diff_type: LineDiffType,
  pub show_background: bool,
  pub hunk_separators: HunkSeparators,
}

impl Default for DiffSettings {
  fn default() -> Self {
    Self {
      layout: DiffLayout::SideBySide,
      show_inline_hunk_actions: false,
      show_line_numbers: true,
      diff_indicators: DiffIndicators::None,
      line_diff_type: LineDiffType::WordAlt,
      show_background: true,
      hunk_separators: HunkSeparators::Simple,
    }
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CursorStyle {
  #[default]
  Block,
  Underline,
  Bar,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum CursorInactiveStyle {
  #[default]
  Outline,
  Block,
  Bar,
  Underline,
  None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum BellStyle {
  #[default]
  Off,
  Sound,
  Visual,
  Both,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminalSettings {
  pub font_size: u32,
  pub font_family: String,
  pub line_height: f32,
  pub cursor_blink: bool,
  pub cursor_style: CursorStyle,
  pub scrollback: u32,
  pub copy_on_select: bool,
  pub cursor_inactive_style: CursorInactiveStyle,
  pub font_weight: String,
  pub font_weight_bold: String,
  pub letter_spacing: f32,
  pub cursor_width: u32,
  pub right_click_selects_word: bool,
  pub mac_option_click_forces_selection: bool,
  pub shell_path: String,
  pub bell_style: BellStyle,
  pub color_saturation: f32,
}

impl Default for TerminalSettings {
  fn default() -> Self {
    Self {
      font_size: 13,
      font_family: MONO_FONT_STACK.to_string(),
      line_height: 1.2,
      cursor_blink: true,
      cursor_style: CursorStyle::Block,
      scrollback: 5000,
      copy_on_select: false,
      cursor_inactive_style: CursorInactiveStyle::Outline,
      font_weight: "normal".to_string(),
      font_weight_bold: "bold".to_string(),
      letter_spacing: 0.0,
      cursor_width: 1,
      right_click_selects_word: false,
      mac_option_click_forces_selection: false,
      shell_path: String::new(),
      bell_style: BellStyle::Off,
      color_saturation: 1.42,
    }
  }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct GitSettings {
  pub blame: bool,
}

impl Default for GitSettings {
  fn default() -> Self {
    Self { blame: true }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct WorkspaceEntry {
  pub directory: String,
  #[serde(default = "default_scan_depth")]
  pub scan_depth: u32,
}

fn default_scan_depth() -> u32 {
  1
}

impl Default for WorkspaceEntry {
  fn default() -> Self {
    Self {
      directory: String::new(),
      scan_depth: default_scan_depth(),
    }
  }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProjectsSettings {
  pub workspaces: Vec<WorkspaceEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ThemeSettings {
  pub current: String,
  pub preferred_dark: String,
  pub preferred_light: String,
}

impl Default for ThemeSettings {
  fn default() -> Self {
    Self {
      current: DEFAULT_DARK_THEME.to_string(),
      preferred_dark: DEFAULT_DARK_THEME.to_string(),
      preferred_light: DEFAULT_LIGHT_THEME.to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn defaults_match_the_settings_spec() {
    let settings = Settings::default();
    assert_eq!(settings.ui.font_size, 13);
    assert_eq!(settings.ui.tree_density, TreeDensity::Compact);
    assert_eq!(settings.ui.tree_icons, TreeIcons::Complete);
    assert_eq!(settings.editor.line_height, 20);
    assert_eq!(settings.diff.layout, DiffLayout::SideBySide);
    assert_eq!(settings.diff.hunk_separators, HunkSeparators::Simple);
    assert_eq!(settings.terminal.scrollback, 5000);
    assert!(!settings.terminal.right_click_selects_word);
    assert!(!settings.terminal.mac_option_click_forces_selection);
    assert!(settings.git.blame);
    assert_eq!(settings.theme.preferred_dark, "warm-burnout-dark");
    assert_eq!(settings.theme.preferred_light, "warm-burnout-light");
  }

  #[test]
  fn workspace_entry_defaults_scan_depth_to_one() {
    assert_eq!(WorkspaceEntry::default().scan_depth, 1);
    let entry: WorkspaceEntry = serde_json::from_str(r#"{"directory":"/tmp/work"}"#).unwrap();
    assert_eq!(entry.directory, "/tmp/work");
    assert_eq!(entry.scan_depth, 1);
  }

  #[test]
  fn partial_json_fills_missing_fields_with_defaults() {
    let settings: Settings = serde_json::from_str(r#"{"ui":{"zoomLevel":2},"diff":{"layout":"inline"}}"#).unwrap();
    assert_eq!(settings.ui.zoom_level, 2);
    assert_eq!(settings.ui.font_size, 13);
    assert_eq!(settings.diff.layout, DiffLayout::Inline);
    assert_eq!(settings.diff.line_diff_type, LineDiffType::WordAlt);
  }

  #[test]
  fn enums_keep_the_old_wire_names() {
    let json = serde_json::to_string(&DiffSettings::default()).unwrap();
    assert!(json.contains("\"lineDiffType\":\"word-alt\""));
    assert!(json.contains("\"hunkSeparators\":\"simple\""));
    let terminal: TerminalSettings =
      serde_json::from_str(r#"{"cursorInactiveStyle":"underline","bellStyle":"both"}"#).unwrap();
    assert_eq!(terminal.cursor_inactive_style, CursorInactiveStyle::Underline);
    assert_eq!(terminal.bell_style, BellStyle::Both);
  }

  #[test]
  fn zoom_scale_clamps() {
    assert!((zoom_scale(0) - 1.0).abs() < f32::EPSILON);
    assert!((zoom_scale(3) - 1.728).abs() < 0.001);
    assert!((zoom_scale(99) - zoom_scale(ZOOM_MAX)).abs() < f32::EPSILON);
    assert!((zoom_scale(-99) - zoom_scale(ZOOM_MIN)).abs() < f32::EPSILON);
  }
}
