use gpui_kit::component::button::Button;
use gpui_kit::component::input::Textarea;
use gpui_kit::component::menu::DropdownMenu;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Disableable, Icon};
use gpui_kit::*;

use super::view::{ChangesChrome, ChangesView};
use crate::actions::{CommitAmendMode, CommitAndPush, CommitAndSync, CommitFromBox};
use crate::config::AppConfig;
use crate::theme::{ActivePalette, hsla};

pub fn commit_tooltip(branch: Option<&str>) -> String {
  let chord = if cfg!(target_os = "macos") {
    "⌘+Enter"
  } else {
    "Ctrl+Enter"
  };
  format!("{chord} to commit on \"{}\"", branch.unwrap_or("HEAD"))
}

pub fn commit_button_tooltip(amend: bool) -> &'static str {
  if amend {
    "Amend staged changes"
  } else {
    "Commit staged changes"
  }
}

pub fn should_sync_commit_message(field: &str, core: &str, field_focused: bool, pending: bool) -> bool {
  field != core && !field_focused && !pending
}

pub fn render_commit_box(
  view: &ChangesView,
  chrome: &ChangesChrome,
  window: &mut Window,
  cx: &mut Context<ChangesView>,
) -> impl IntoElement {
  let _ = window;
  let palette = cx.global::<ActivePalette>().0;
  let font = AppConfig::get(cx).settings.editor.font_family.clone();
  let field_tooltip = commit_tooltip(chrome.head_branch.as_deref());
  let actions = chrome.actions.as_ref();
  let can_commit = actions.is_some_and(|actions| actions.can_commit);
  let label = actions
    .map(|actions| actions.commit_label.clone())
    .unwrap_or_else(|| "Commit".into());
  let committing = view.committing;
  let disabled = !can_commit || committing;
  let button_tooltip = commit_button_tooltip(chrome.amend_mode);

  div().px_2().pb_2().child(
    div()
      .p(px(8.0))
      .bg(hsla(palette.input))
      .rounded_md()
      .child(
        div()
          .id("commit-message")
          .tooltip(move |window, cx| Tooltip::new(field_tooltip.clone()).build(window, cx))
          .child(
            Textarea::new(&view.commit)
              .font_family(SharedString::from(font))
              .min_h(px(42.0))
              .max_h(px(180.0)),
          ),
      )
      .child(
        div()
          .mt_2()
          .flex()
          .items_center()
          .h(px(26.0))
          .child(
            Button::new("commit")
              .outline()
              .flex_1()
              .h(px(26.0))
              .icon(Icon::empty().path("icons/check.svg"))
              .label(label)
              .tooltip(button_tooltip)
              .loading(committing)
              .disabled(disabled)
              .on_click(cx.listener(|this, _, window, cx| this.commit(window, cx))),
          )
          .child(div().w(px(1.0)).h_full().bg(hsla(palette.border)))
          .child(
            Button::new("commit-more")
              .outline()
              .w(px(26.0))
              .h(px(26.0))
              .icon(Icon::empty().path("icons/chevron-down.svg"))
              .tooltip("More commit options")
              .disabled(disabled)
              .dropdown_menu(move |menu, _, _| {
                menu
                  .menu_with_disabled("Commit", Box::new(CommitFromBox), !can_commit)
                  .menu_with_disabled("Commit (Amend)", Box::new(CommitAmendMode), !can_commit)
                  .menu_with_disabled("Commit & Push", Box::new(CommitAndPush), !can_commit)
                  .menu_with_disabled("Commit & Sync", Box::new(CommitAndSync), !can_commit)
                  .min_w(px(220.0))
              }),
          ),
      ),
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn should_sync_skips_when_focused_pending_or_equal() {
    assert!(should_sync_commit_message("typed", "core", false, false));
    assert!(!should_sync_commit_message("typed", "core", true, false));
    assert!(!should_sync_commit_message("typed", "core", false, true));
    assert!(!should_sync_commit_message("typed", "core", true, true));
    assert!(!should_sync_commit_message("same", "same", false, false));
  }

  #[test]
  fn tooltips_name_the_branch_and_the_chord() {
    let chord = if cfg!(target_os = "macos") {
      "⌘+Enter"
    } else {
      "Ctrl+Enter"
    };
    assert_eq!(commit_tooltip(Some("main")), format!("{chord} to commit on \"main\""));
    assert_eq!(commit_tooltip(None), format!("{chord} to commit on \"HEAD\""));
    assert_eq!(commit_button_tooltip(false), "Commit staged changes");
    assert_eq!(commit_button_tooltip(true), "Amend staged changes");
  }
}
