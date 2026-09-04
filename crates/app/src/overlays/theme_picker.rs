use deathpush_core::config::settings::ThemeSettings;
use deathpush_core::theme::ThemeKind;
use gpui_kit::base::actions::{SelectDown, SelectUp};
use gpui_kit::component::IndexPath;
use gpui_kit::component::command::{Command, CommandGroup, CommandItem, CommandState};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::frame::backdrop;
use crate::actions::Cancel;
use crate::config::AppConfig;
use crate::keymap::CONTEXT_DIALOG;
use crate::theme::{ActivePalette, ThemeCatalog, ThemeEntry, commit_theme, hsla, preview_theme, restore_theme};

const PLACEHOLDER: &str = "Select Color Theme";
const DARK_THEMES: &str = "dark themes";
const LIGHT_THEMES: &str = "light themes";

/// Display name, or the id title-cased on hyphens when the label is empty.
pub fn theme_label(entry: &ThemeEntry) -> String {
  if entry.label.is_empty() {
    title_case(&entry.id)
  } else {
    entry.label.clone()
  }
}

fn title_case(id: &str) -> String {
  id.split('-')
    .map(|part| {
      let mut chars = part.chars();
      match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
      }
    })
    .collect::<Vec<_>>()
    .join(" ")
}

/// Filter by label substring and put the OS-preferred kind first.
pub fn grouped(entries: &[ThemeEntry], query: &str, os_dark: bool) -> (Vec<ThemeEntry>, Vec<ThemeEntry>) {
  let needle = query.trim().to_lowercase();
  let matches = |entry: &ThemeEntry| needle.is_empty() || theme_label(entry).to_lowercase().contains(&needle);
  let dark: Vec<ThemeEntry> = entries
    .iter()
    .filter(|entry| entry.kind == ThemeKind::Dark && matches(entry))
    .cloned()
    .collect();
  let light: Vec<ThemeEntry> = entries
    .iter()
    .filter(|entry| entry.kind == ThemeKind::Light && matches(entry))
    .cloned()
    .collect();
  if os_dark { (dark, light) } else { (light, dark) }
}

/// Set `current` and the preferred theme of `kind`.
pub fn preferred_update(kind: ThemeKind, id: &str, settings: &mut ThemeSettings) {
  settings.current = id.to_string();
  match kind {
    ThemeKind::Dark => settings.preferred_dark = id.to_string(),
    ThemeKind::Light => settings.preferred_light = id.to_string(),
  }
}

fn index_path_of(first: &[ThemeEntry], second: &[ThemeEntry], id: &str) -> Option<IndexPath> {
  let mut section = 0;
  for group in [first, second] {
    if group.is_empty() {
      continue;
    }
    if let Some(row) = group.iter().position(|entry| entry.id == id) {
      return Some(IndexPath::new(row).section(section));
    }
    section += 1;
  }
  None
}

/// Close the overlay.
pub enum ThemePickerEvent {
  Close,
}

/// Color theme command palette.
pub struct ThemePicker {
  state: Entity<CommandState>,
  entries: Vec<ThemeEntry>,
  query: String,
  original_id: String,
  os_dark: bool,
}

impl EventEmitter<ThemePickerEvent> for ThemePicker {}

impl ThemePicker {
  pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
    let os_dark = matches!(
      cx.window_appearance(),
      WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    let entries = ThemeCatalog::get(cx).entries.clone();
    let original_id = AppConfig::get(cx).settings.theme.current.clone();
    let state = cx.new(|cx| CommandState::new(window, cx));
    state.update(cx, |state, cx| state.focus(window, cx));
    let current = original_id.clone();
    cx.defer_in(window, move |this, window, cx| {
      let (first, second) = grouped(&this.entries, "", this.os_dark);
      if let Some(path) = index_path_of(&first, &second, &current) {
        this.state.update(cx, |state, cx| {
          state.set_selected_index(Some(path), window, cx);
        });
      }
    });
    Self {
      state,
      entries,
      query: String::new(),
      original_id,
      os_dark,
    }
  }

  pub fn focus(&self, window: &mut Window, cx: &mut Context<Self>) {
    self.state.update(cx, |state, cx| state.focus(window, cx));
  }

  fn groups(&self) -> (Vec<ThemeEntry>, Vec<ThemeEntry>) {
    grouped(&self.entries, &self.query, self.os_dark)
  }

  fn entry_at(&self, path: IndexPath) -> Option<ThemeEntry> {
    let (first, second) = self.groups();
    match (first.is_empty(), second.is_empty(), path.section) {
      (false, _, 0) => first.get(path.row).cloned(),
      (false, false, 1) => second.get(path.row).cloned(),
      (true, false, 0) => second.get(path.row).cloned(),
      _ => None,
    }
  }

  fn preview_at(&self, path: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(entry) = self.entry_at(path) {
      preview_theme(&entry.id, window, cx);
    }
  }

  fn schedule_preview(&self, window: &mut Window, cx: &mut Context<Self>) {
    let state = self.state.clone();
    let this = cx.entity().downgrade();
    window.defer(cx, move |window, cx| {
      let Some(path) = state.read(cx).selected_index() else {
        return;
      };
      let _ = this.update(cx, |this, cx| this.preview_at(path, window, cx));
    });
  }

  fn on_query_change(&mut self, query: &str, window: &mut Window, cx: &mut Context<Self>) {
    self.query = query.to_string();
    let (first, second) = grouped(&self.entries, &self.query, self.os_dark);
    if let Some(id) = first.first().or(second.first()).map(|entry| entry.id.clone()) {
      preview_theme(&id, window, cx);
    }
    cx.notify();
  }

  fn confirm(&mut self, path: IndexPath, window: &mut Window, cx: &mut Context<Self>) {
    let Some(entry) = self.entry_at(path) else {
      return;
    };
    preview_theme(&entry.id, window, cx);
    commit_theme(&entry.id, cx);
    self.close(cx);
  }

  fn cancel(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    restore_theme(&self.original_id, window, cx);
    self.close(cx);
  }

  fn close(&mut self, cx: &mut Context<Self>) {
    cx.emit(ThemePickerEvent::Close);
  }
}

fn command_items(entries: &[ThemeEntry]) -> Vec<CommandItem> {
  entries
    .iter()
    .map(|entry| CommandItem::new().label(theme_label(entry)))
    .collect()
}

impl Render for ThemePicker {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let this = cx.entity().downgrade();
    let (first, second) = self.groups();
    let (first_label, second_label) = if self.os_dark {
      (DARK_THEMES, LIGHT_THEMES)
    } else {
      (LIGHT_THEMES, DARK_THEMES)
    };
    let on_query = this.clone();
    let on_confirm = this.clone();
    let on_cancel = this;
    let mut command = Command::new(&self.state)
      .filterable(false)
      .placeholder(PLACEHOLDER)
      .max_h(px(440.))
      .bordered(false)
      .w_full()
      .bg(hsla(palette.sidebar))
      .text_size(px(13.))
      .on_query(move |query, window, cx| {
        let query = query.to_string();
        let _ = on_query.update(cx, |this, cx| this.on_query_change(&query, window, cx));
      })
      .on_confirm(move |index, window, cx| {
        let _ = on_confirm.update(cx, |this, cx| this.confirm(index, window, cx));
      })
      .on_cancel(move |window, cx| {
        let _ = on_cancel.update(cx, |this, cx| this.cancel(window, cx));
      });
    if !first.is_empty() {
      command = command.group(CommandGroup::new().label(first_label).items(command_items(&first)));
    }
    if !first.is_empty() && !second.is_empty() {
      command = command.separator();
    }
    if !second.is_empty() {
      command = command.group(CommandGroup::new().label(second_label).items(command_items(&second)));
    }
    backdrop("theme-picker-backdrop", |_, _| {}, cx)
      .on_mouse_down(
        MouseButton::Left,
        cx.listener(|this, _, window, cx| this.cancel(window, cx)),
      )
      .child(
        div()
          .key_context(CONTEXT_DIALOG)
          .occlude()
          .mt(px(60.))
          .w(px(500.))
          .max_h(px(440.))
          .bg(hsla(palette.sidebar))
          .border_1()
          .border_color(hsla(palette.border))
          .rounded_lg()
          .shadow_lg()
          .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
          .capture_action(cx.listener(|this, _: &SelectUp, window, cx| this.schedule_preview(window, cx)))
          .capture_action(cx.listener(|this, _: &SelectDown, window, cx| this.schedule_preview(window, cx)))
          .on_action(cx.listener(|this, _: &Cancel, window, cx| this.cancel(window, cx)))
          .child(command),
      )
  }
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

  fn ids(entries: &[ThemeEntry]) -> Vec<&str> {
    entries.iter().map(|entry| entry.id.as_str()).collect()
  }

  #[test]
  fn theme_label_fallback() {
    assert_eq!(theme_label(&entry("vesper", "Vesper", ThemeKind::Dark)), "Vesper");
    assert_eq!(theme_label(&entry("ayu-light", "", ThemeKind::Light)), "Ayu Light");
  }

  #[test]
  fn grouped_follows_os_scheme_and_filters() {
    let entries = vec![
      entry("vesper", "Vesper", ThemeKind::Dark),
      entry("ayu-light", "Ayu Light", ThemeKind::Light),
      entry("one-dark", "One Dark", ThemeKind::Dark),
    ];

    let (first, second) = grouped(&entries, "", true);
    assert_eq!(ids(&first), ["vesper", "one-dark"]);
    assert_eq!(ids(&second), ["ayu-light"]);

    let (first, second) = grouped(&entries, "", false);
    assert_eq!(ids(&first), ["ayu-light"]);
    assert_eq!(ids(&second), ["vesper", "one-dark"]);

    let (first, second) = grouped(&entries, "ay", true);
    assert!(first.is_empty());
    assert_eq!(ids(&second), ["ayu-light"]);

    let (first, second) = grouped(&entries, "DARK", false);
    assert!(first.is_empty());
    assert_eq!(ids(&second), ["one-dark"]);
  }

  #[test]
  fn preferred_update_sets_kind() {
    let mut settings = ThemeSettings {
      current: "vesper".into(),
      preferred_dark: "vesper".into(),
      preferred_light: "ayu-light".into(),
    };
    preferred_update(ThemeKind::Light, "github-light", &mut settings);
    assert_eq!(settings.current, "github-light");
    assert_eq!(settings.preferred_light, "github-light");
    assert_eq!(settings.preferred_dark, "vesper");
    preferred_update(ThemeKind::Dark, "one-dark", &mut settings);
    assert_eq!(settings.current, "one-dark");
    assert_eq!(settings.preferred_dark, "one-dark");
    assert_eq!(settings.preferred_light, "github-light");
  }
}
