use deathpush_core::config::settings::{
  DiffIndicators, DiffLayout, DiffSettings, EditorSettings, GitSettings, HunkSeparators, LineDiffType, Settings,
  SidebarPosition, TreeDensity, TreeIcons, WordWrap,
};
use deathpush_core::config::settings_ui::workspace_summary;
use deathpush_core::theme::ThemeKind;
use gpui_kit::component::input::InputState;
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

const CLASSIC_INDICATORS: &str = "Classic (+\u{2212})";

pub fn themes_of_kind(entries: &[ThemeEntry], kind: ThemeKind) -> Vec<(SharedString, String)> {
  entries
    .iter()
    .filter(|entry| entry.kind == kind)
    .map(|entry| (SharedString::from(theme_label(entry)), entry.id.clone()))
    .collect()
}

pub fn appearance(
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
      "UI Font Size",
      ui.font_size as f64,
      10.0,
      20.0,
      1.0,
      persist(view.clone(), |value: f64, cx| {
        AppConfig::update(cx, |c| c.settings.ui.font_size = value.round() as u32);
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
      persist(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.ui.always_open_terminal_on_start = value);
      }),
    ))
}

pub fn editor(
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

pub fn diff_viewer(diff: &DiffSettings, view: WeakEntity<SettingsView>, cx: &App) -> impl IntoElement {
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
      persist(view.clone(), |value, cx| {
        AppConfig::update(cx, |c| c.settings.diff.show_inline_hunk_actions = value);
      }),
    ))
    .child(toggle_row(
      "Line Numbers",
      diff.show_line_numbers,
      persist(view.clone(), |value, cx| {
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
      persist(view.clone(), |value, cx| {
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

pub fn git(
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
    .child(toggle_row(
      "Git Blame",
      git.blame,
      persist(view, |value, cx| {
        AppConfig::update(cx, |c| c.settings.git.blame = value);
      }),
    ))
    .child(text_row("User Name", name_input))
    .child(text_row("User Email", email_input))
}

pub fn projects(settings: &Settings, cx: &App) -> impl IntoElement {
  let summary = workspace_summary(&settings.projects.workspaces).unwrap_or_else(|| "Not configured".into());
  div()
    .flex()
    .flex_col()
    .gap_1()
    .child(section_title("Projects", cx))
    .child(projects_row(&summary, cx))
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
