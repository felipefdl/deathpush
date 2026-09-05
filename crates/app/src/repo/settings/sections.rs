use deathpush_core::config::settings::{
  BellStyle, CursorInactiveStyle, CursorStyle, DiffIndicators, DiffLayout, DiffSettings, EditorSettings, GitSettings,
  HunkSeparators, LineDiffType, Settings, SidebarPosition, TerminalSettings, TreeDensity, TreeIcons, WordWrap,
};
use deathpush_core::config::settings_ui::{
  FONT_WEIGHTS, ShellPreset, preset_for, shell_exists, shell_presets, workspace_summary,
};
use deathpush_core::theme::ThemeKind;
use gpui_kit::component::input::InputState;
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::rows::{
  color_theme_button, color_theme_hint, number_row, projects_row, section_title, select_row, text_row, toggle_row,
  zoom_select_options,
};
use super::view::SettingsView;
use crate::config::AppConfig;
use crate::overlays::theme_picker::theme_label;
use crate::theme::{ThemeCatalog, ThemeEntry};
use crate::zoom;

const CLASSIC_INDICATORS: &str = "Classic (+/\u{2212})";

pub(crate) fn themes_of_kind(entries: &[ThemeEntry], kind: ThemeKind) -> Vec<(SharedString, String)> {
  entries
    .iter()
    .filter(|entry| entry.kind == kind)
    .map(|entry| (SharedString::from(theme_label(entry)), entry.id.clone()))
    .collect()
}

pub(crate) fn appearance(
  settings: &Settings,
  catalog: &[ThemeEntry],
  ui_font: &Entity<InputState>,
  view: WeakEntity<SettingsView>,
  cx: &App,
) -> impl IntoElement {
  let ui = &settings.ui;
  let current_label = catalog
    .iter()
    .find(|entry| entry.id == settings.theme.current)
    .map(theme_label)
    .unwrap_or_else(|| settings.theme.current.clone());
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Appearance", cx))
    .child(color_theme_button(current_label, color_theme_hint(), cx))
    .child(select_row(
      "Preferred Dark Theme",
      themes_of_kind(catalog, ThemeKind::Dark),
      settings.theme.preferred_dark.clone(),
      persist(view.clone(), |id, cx| set_preferred(ThemeKind::Dark, id, cx)),
    ))
    .child(select_row(
      "Preferred Light Theme",
      themes_of_kind(catalog, ThemeKind::Light),
      settings.theme.preferred_light.clone(),
      persist(view.clone(), |id, cx| set_preferred(ThemeKind::Light, id, cx)),
    ))
    .child(select_row(
      "Tree Density",
      tree_density_options(),
      ui.tree_density,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.ui.tree_density = value);
      }),
    ))
    .child(select_row(
      "Tree Icons",
      tree_icons_options(),
      ui.tree_icons,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.ui.tree_icons = value);
      }),
    ))
    .child(select_row(
      "Sidebar Position",
      sidebar_position_options(),
      ui.sidebar_position,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.ui.sidebar_position = value);
      }),
    ))
    .child(text_row("UI Font Family", ui_font))
    .child(number_row(
      "ui-font-size",
      "UI Font Size",
      ui.font_size as f64,
      10.0,
      20.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.ui.font_size = value.round() as u32);
        crate::theme::refresh_ui_font(None, cx);
      }),
    ))
    .child(select_row(
      "Zoom",
      zoom_select_options(),
      zoom::current_level(cx),
      persist(view.clone(), zoom::set_zoom_level),
    ))
    .child(toggle_row(
      "Always Open Terminal on Start",
      ui.always_open_terminal_on_start,
      persist_click(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.ui.always_open_terminal_on_start = value);
      }),
    ))
}

pub(crate) fn editor(
  editor: &EditorSettings,
  font_input: &Entity<InputState>,
  view: WeakEntity<SettingsView>,
  cx: &App,
) -> impl IntoElement {
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Editor", cx))
    .child(number_row(
      "editor-font-size",
      "Font Size",
      editor.font_size as f64,
      8.0,
      32.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.editor.font_size = value.round() as u32);
      }),
    ))
    .child(text_row("Font Family", font_input))
    .child(number_row(
      "editor-line-height",
      "Line Height",
      editor.line_height as f64,
      10.0,
      60.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.editor.line_height = value.round() as u32);
      }),
    ))
    .child(number_row(
      "editor-tab-size",
      "Tab Size",
      editor.tab_size as f64,
      1.0,
      8.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.editor.tab_size = value.round() as u32);
      }),
    ))
    .child(select_row(
      "Word Wrap",
      word_wrap_options(),
      editor.word_wrap,
      persist(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.editor.word_wrap = value);
      }),
    ))
}

pub(crate) fn diff_viewer(diff: &DiffSettings, view: WeakEntity<SettingsView>, cx: &App) -> impl IntoElement {
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Diff Viewer", cx))
    .child(select_row(
      "Diff Layout",
      diff_layout_options(),
      diff.layout,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.layout = value);
      }),
    ))
    .child(toggle_row(
      "Inline Hunk Actions",
      diff.show_inline_hunk_actions,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.show_inline_hunk_actions = value);
      }),
    ))
    .child(toggle_row(
      "Line Numbers",
      diff.show_line_numbers,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.show_line_numbers = value);
      }),
    ))
    .child(select_row(
      "Diff Indicators",
      diff_indicators_options(),
      diff.diff_indicators,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.diff_indicators = value);
      }),
    ))
    .child(select_row(
      "Inline Changes",
      line_diff_options(),
      diff.line_diff_type,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.line_diff_type = value);
      }),
    ))
    .child(toggle_row(
      "Background Highlighting",
      diff.show_background,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.show_background = value);
      }),
    ))
    .child(select_row(
      "Hunk Separators",
      hunk_separators_options(),
      diff.hunk_separators,
      persist(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.hunk_separators = value);
      }),
    ))
}

pub(crate) fn git(
  git: &GitSettings,
  name_input: &Entity<InputState>,
  email_input: &Entity<InputState>,
  view: WeakEntity<SettingsView>,
  cx: &App,
) -> impl IntoElement {
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Git", cx))
    .child(toggle_row("Git Blame", git.blame, move |value, window, cx| {
      let _ = view.update(cx, |this, cx| this.set_git_blame(value, window, cx));
    }))
    .child(text_row("User Name", name_input))
    .child(text_row("User Email", email_input))
}

pub(crate) fn projects(settings: &Settings, cx: &App) -> impl IntoElement {
  let summary = workspace_summary(&settings.projects.workspaces).unwrap_or_else(|| "Not configured".into());
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Projects", cx))
    .child(projects_row(&summary, cx))
}

pub(crate) fn terminal(
  terminal: &TerminalSettings,
  font_input: &Entity<InputState>,
  shell_path_input: &Entity<InputState>,
  custom_selected: bool,
  view: WeakEntity<SettingsView>,
  cx: &App,
) -> impl IntoElement {
  let presets = shell_presets(&|path| env_shell_exists(path));
  let stored = preset_for(&terminal.shell_path, &presets);
  let current = if custom_selected || custom_shell_visible(&stored) {
    ShellPreset::Custom
  } else {
    stored
  };
  let show_custom = custom_shell_visible(&current);
  let shell_input = shell_path_input.clone();
  let shell_view = view.clone();
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Terminal", cx))
    .child(section_title("Text & Font", cx))
    .child(number_row(
      "terminal-font-size",
      "Font Size",
      terminal.font_size as f64,
      8.0,
      32.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.font_size = value.round() as u32);
      }),
    ))
    .child(text_row("Font Family", font_input))
    .child(number_row(
      "terminal-line-height",
      "Line Height",
      terminal.line_height as f64,
      0.8,
      3.0,
      0.1,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| {
          c.settings.terminal.line_height = ((value * 10.0).round() / 10.0) as f32;
        });
      }),
    ))
    .child(select_row(
      "Font Weight",
      font_weight_options(),
      terminal.font_weight.clone(),
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.font_weight = value);
      }),
    ))
    .child(select_row(
      "Font Weight Bold",
      font_weight_options(),
      terminal.font_weight_bold.clone(),
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.font_weight_bold = value);
      }),
    ))
    .child(number_row(
      "terminal-letter-spacing",
      "Letter Spacing",
      terminal.letter_spacing as f64,
      -5.0,
      10.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.letter_spacing = value.round() as f32);
      }),
    ))
    .child(section_title("Cursor", cx))
    .child(select_row(
      "Cursor Style",
      cursor_style_options(),
      terminal.cursor_style,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.cursor_style = value);
      }),
    ))
    .child(toggle_row(
      "Cursor Blink",
      terminal.cursor_blink,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.cursor_blink = value);
      }),
    ))
    .child(number_row(
      "terminal-cursor-width",
      "Cursor Width",
      terminal.cursor_width as f64,
      1.0,
      5.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.cursor_width = value.round() as u32);
      }),
    ))
    .child(select_row(
      "Cursor Inactive Style",
      cursor_inactive_options(),
      terminal.cursor_inactive_style,
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.cursor_inactive_style = value);
      }),
    ))
    .child(section_title("Scrolling", cx))
    .child(number_row(
      "terminal-scrollback",
      "Scrollback for New Terminals (KiB)",
      terminal.scrollback as f64,
      500.0,
      100_000.0,
      500.0,
      persist(view.clone(), |value: f64, cx| {
        let n = ((value / 500.0).round() * 500.0) as u32;
        AppConfig::update(cx, |c| c.settings.terminal.scrollback = n.clamp(500, 100_000));
      }),
    ))
    .child(section_title("Behavior", cx))
    .child(toggle_row(
      "Copy on Select",
      terminal.copy_on_select,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.copy_on_select = value);
      }),
    ))
    .child(toggle_row(
      "Right Click Selects Word",
      terminal.right_click_selects_word,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.right_click_selects_word = value);
      }),
    ))
    .child(toggle_row(
      "macOS Option Click Forces Selection",
      terminal.mac_option_click_forces_selection,
      persist_click(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.mac_option_click_forces_selection = value);
      }),
    ))
    .child(section_title("Rendering", cx))
    .child(number_row(
      "terminal-color-saturation",
      "Color Saturation",
      terminal.color_saturation as f64,
      0.5,
      2.0,
      0.01,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| {
          c.settings.terminal.color_saturation = ((value * 100.0).round() / 100.0) as f32;
        });
      }),
    ))
    .child(section_title("Shell", cx))
    .child(select_row(
      "Shell Path",
      shell_preset_options(&presets),
      current,
      persist(view.clone(), move |preset, cx| {
        let custom = shell_input.read(cx).value().to_string();
        let _ = shell_view.update(cx, |this, _| {
          this.shell_custom = matches!(preset, ShellPreset::Custom);
        });
        AppConfig::update(cx, |c| {
          c.settings.terminal.shell_path = stored_shell_path(&preset, &custom);
        });
      }),
    ))
    .when(show_custom, |el| el.child(text_row("", shell_path_input)))
    .child(select_row(
      "Bell Style",
      bell_style_options(),
      terminal.bell_style,
      persist(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.terminal.bell_style = value);
      }),
    ))
}

/// `shell_exists` using this process `PATH` and, on Windows, `PATHEXT`.
pub(crate) fn env_shell_exists(path: &str) -> bool {
  let env_path = std::env::var("PATH").ok();
  let path_ext = if cfg!(windows) {
    std::env::var("PATHEXT").ok()
  } else {
    None
  };
  shell_exists(path, env_path.as_deref(), path_ext.as_deref())
}

/// Stored `shell_path` for a selected preset. Custom keeps the typed path.
pub(crate) fn stored_shell_path(preset: &ShellPreset, custom: &str) -> String {
  match preset {
    ShellPreset::Default => String::new(),
    ShellPreset::Path(path) => path.clone(),
    ShellPreset::Custom => custom.to_string(),
  }
}

/// The custom path field shows only when the selected preset is Custom.
pub(crate) fn custom_shell_visible(preset: &ShellPreset) -> bool {
  matches!(preset, ShellPreset::Custom)
}

fn persist<T: 'static>(
  view: WeakEntity<SettingsView>,
  mutate: impl Fn(T, &mut App) + 'static,
) -> impl Fn(T, &mut App) + 'static {
  move |value, cx| {
    mutate(value, cx);
    let _ = view.update(cx, |_, cx| cx.notify());
  }
}

fn persist_click(
  view: WeakEntity<SettingsView>,
  mutate: impl Fn(bool, &mut App) + 'static,
) -> impl Fn(bool, &mut Window, &mut App) + 'static {
  let inner = persist(view, mutate);
  move |value, _, cx| inner(value, cx)
}

fn set_preferred(kind: ThemeKind, id: String, cx: &mut App) {
  let current_kind = ThemeCatalog::get(cx)
    .kind(&AppConfig::get(cx).settings.theme.current)
    .unwrap_or(ThemeKind::Dark);
  AppConfig::update(cx, |c| match kind {
    ThemeKind::Dark => c.settings.theme.preferred_dark = id.clone(),
    ThemeKind::Light => c.settings.theme.preferred_light = id.clone(),
  });
  if current_kind == kind {
    crate::theme::apply_theme(&id, kind, None, cx);
  }
}

fn tree_density_options() -> Vec<(SharedString, TreeDensity)> {
  vec![
    ("Compact".into(), TreeDensity::Compact),
    ("Default".into(), TreeDensity::Default),
    ("Relaxed".into(), TreeDensity::Relaxed),
  ]
}

fn tree_icons_options() -> Vec<(SharedString, TreeIcons)> {
  vec![
    ("Minimal".into(), TreeIcons::Minimal),
    ("Standard".into(), TreeIcons::Standard),
    ("Complete".into(), TreeIcons::Complete),
  ]
}

fn sidebar_position_options() -> Vec<(SharedString, SidebarPosition)> {
  vec![
    ("Left".into(), SidebarPosition::Left),
    ("Right".into(), SidebarPosition::Right),
  ]
}

fn word_wrap_options() -> Vec<(SharedString, WordWrap)> {
  vec![("Off".into(), WordWrap::Off), ("On".into(), WordWrap::On)]
}

fn diff_layout_options() -> Vec<(SharedString, DiffLayout)> {
  vec![
    ("Side by Side".into(), DiffLayout::SideBySide),
    ("Inline".into(), DiffLayout::Inline),
  ]
}

fn diff_indicators_options() -> Vec<(SharedString, DiffIndicators)> {
  vec![
    ("None".into(), DiffIndicators::None),
    ("Bars".into(), DiffIndicators::Bars),
    (CLASSIC_INDICATORS.into(), DiffIndicators::Classic),
  ]
}

fn line_diff_options() -> Vec<(SharedString, LineDiffType)> {
  vec![
    ("Smart Words".into(), LineDiffType::WordAlt),
    ("Words".into(), LineDiffType::Word),
    ("Characters".into(), LineDiffType::Char),
    ("None".into(), LineDiffType::None),
  ]
}

fn hunk_separators_options() -> Vec<(SharedString, HunkSeparators)> {
  vec![
    ("Compact Line Info".into(), HunkSeparators::LineInfoBasic),
    ("Line Info".into(), HunkSeparators::LineInfo),
    ("Metadata".into(), HunkSeparators::Metadata),
    ("Simple".into(), HunkSeparators::Simple),
  ]
}

fn font_weight_options() -> Vec<(SharedString, String)> {
  FONT_WEIGHTS
    .iter()
    .map(|(label, value)| ((*label).into(), (*value).to_string()))
    .collect()
}

fn cursor_style_options() -> Vec<(SharedString, CursorStyle)> {
  vec![
    ("Block".into(), CursorStyle::Block),
    ("Underline".into(), CursorStyle::Underline),
    ("Bar".into(), CursorStyle::Bar),
  ]
}

fn cursor_inactive_options() -> Vec<(SharedString, CursorInactiveStyle)> {
  vec![
    ("Outline".into(), CursorInactiveStyle::Outline),
    ("Block".into(), CursorInactiveStyle::Block),
    ("Bar".into(), CursorInactiveStyle::Bar),
    ("Underline".into(), CursorInactiveStyle::Underline),
    ("None".into(), CursorInactiveStyle::None),
  ]
}

fn bell_style_options() -> Vec<(SharedString, BellStyle)> {
  vec![
    ("Off".into(), BellStyle::Off),
    ("Sound".into(), BellStyle::Sound),
    ("Visual".into(), BellStyle::Visual),
    ("Both".into(), BellStyle::Both),
  ]
}

fn shell_preset_options(presets: &[ShellPreset]) -> Vec<(SharedString, ShellPreset)> {
  presets
    .iter()
    .map(|preset| (SharedString::from(preset.label()), preset.clone()))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  fn entry(id: &str, label: &str, kind: ThemeKind) -> ThemeEntry {
    ThemeEntry {
      id: id.into(),
      label: label.into(),
      kind,
    }
  }

  #[test]
  fn shell_preset_maps_to_stored_path() {
    assert_eq!(stored_shell_path(&ShellPreset::Default, "/custom"), "");
    assert_eq!(
      stored_shell_path(&ShellPreset::Path("/bin/zsh".into()), "/custom"),
      "/bin/zsh"
    );
    assert_eq!(stored_shell_path(&ShellPreset::Custom, "/custom"), "/custom");
    assert!(!custom_shell_visible(&ShellPreset::Default));
    assert!(!custom_shell_visible(&ShellPreset::Path("/bin/zsh".into())));
    assert!(custom_shell_visible(&ShellPreset::Custom));
  }

  #[test]
  fn classic_indicators_use_slash_and_minus_sign() {
    assert_eq!(CLASSIC_INDICATORS, "Classic (+/\u{2212})");
    assert!(CLASSIC_INDICATORS.contains('/'));
    assert!(CLASSIC_INDICATORS.contains('\u{2212}'));
  }

  #[test]
  fn theme_selects_filter_by_kind() {
    let entries = [
      entry("vesper", "Vesper", ThemeKind::Dark),
      entry("ayu-light", "Ayu Light", ThemeKind::Light),
      entry("one-dark", "", ThemeKind::Dark),
    ];
    let dark = themes_of_kind(&entries, ThemeKind::Dark);
    let light = themes_of_kind(&entries, ThemeKind::Light);
    assert_eq!(
      dark
        .iter()
        .map(|(label, id)| (label.as_ref(), id.as_str()))
        .collect::<Vec<_>>(),
      vec![("Vesper", "vesper"), ("One Dark", "one-dark")]
    );
    assert_eq!(
      light
        .iter()
        .map(|(label, id)| (label.as_ref(), id.as_str()))
        .collect::<Vec<_>>(),
      vec![("Ayu Light", "ayu-light")]
    );
  }
}
