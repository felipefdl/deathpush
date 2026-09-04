use chrono::Utc;
use deathpush_core::ops::blame_status_line;
use deathpush_core::relative_time::relative_time;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::*;

use crate::actions::{ShowBranchPicker, ShowHistory, ZoomReset};
use crate::config::AppConfig;
use crate::repo::state::RepoState;
use crate::theme::{ActivePalette, hsla};
use crate::zoom;

/// `{behind}↓ {ahead}↑`, or None when both are zero.
pub fn sync_badge(ahead: usize, behind: usize) -> Option<String> {
  (ahead > 0 || behind > 0).then(|| format!("{behind}↓ {ahead}↑"))
}

/// The first line of a commit message, cut to `max` characters with an ellipsis.
pub fn truncate_message(message: &str, max: usize) -> String {
  let first = message.lines().next().unwrap_or("").trim();
  if first.chars().count() <= max {
    first.to_string()
  } else {
    let cut: String = first.chars().take(max.saturating_sub(1)).collect();
    format!("{cut}…")
  }
}

pub fn render_status_bar(state: &RepoState, window: &mut Window, cx: &App) -> impl IntoElement {
  let palette = cx.global::<ActivePalette>().0;
  let branch = state
    .head_branch()
    .map(str::to_string)
    .unwrap_or_else(|| "No branch".to_string());
  let badge = state
    .status
    .as_ref()
    .and_then(|status| sync_badge(status.ahead, status.behind));
  let blame = (AppConfig::get(cx).settings.git.blame)
    .then(|| {
      let line = state.cursor_line?;
      blame_status_line(state.blame.as_ref()?, line, Utc::now())
    })
    .flatten();
  let zoom_level = zoom::current_level(cx);
  let last_commit = state.last_commit.as_ref().map(|commit| {
    (
      truncate_message(&commit.message, 60),
      relative_time(&commit.author_date, Utc::now()),
    )
  });
  let item = |id: &'static str| {
    div()
      .id(id)
      .flex()
      .items_center()
      .gap_1()
      .h_full()
      .px_2()
      .cursor_pointer()
      .hover(|el| el.bg(hsla(palette.list_hover)))
  };
  let _ = window;
  div()
    .h(px(22.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .text_size(px(12.0))
    .bg(hsla(palette.status_bar))
    .text_color(hsla(palette.status_bar_foreground))
    .border_t_1()
    .border_color(hsla(palette.border))
    .child(
      item("status-branch")
        .child(
          svg()
            .path("icons/source-control.svg")
            .size(px(14.0))
            .text_color(hsla(palette.status_bar_foreground)),
        )
        .child(branch)
        .tooltip(|window, cx| Tooltip::new("Switch branch").build(window, cx))
        .on_click(|_, window, cx| window.dispatch_action(Box::new(ShowBranchPicker), cx)),
    )
    .children(badge.map(|text| div().px_2().child(text)))
    .children(blame.map(|text| {
      div()
        .px_2()
        .text_size(px(12.0))
        .text_color(hsla(palette.muted_foreground))
        .child(text)
    }))
    .child(div().flex_1())
    .children((zoom_level != 0).then(|| {
      item("status-zoom")
        .child(format!("{}%", zoom::zoom_percent(zoom_level)))
        .tooltip(|window, cx| Tooltip::new("Reset Zoom").build(window, cx))
        .on_click(|_, window, cx| window.dispatch_action(Box::new(ZoomReset), cx))
    }))
    .children(last_commit.map(|(message, when)| {
      item("status-last-commit")
        .max_w(px(420.0))
        .child(
          svg()
            .path("icons/git-commit.svg")
            .size(px(14.0))
            .text_color(hsla(palette.status_bar_foreground)),
        )
        .child(div().truncate().child(message))
        .child(div().text_color(hsla(palette.muted_foreground)).child(when))
        .tooltip(|window, cx| Tooltip::new("View history").build(window, cx))
        .on_click(|_, window, cx| window.dispatch_action(Box::new(ShowHistory), cx))
    }))
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn badge_hides_when_both_are_zero() {
    assert_eq!(sync_badge(0, 0), None);
    assert_eq!(sync_badge(2, 1).as_deref(), Some("1↓ 2↑"));
  }

  #[test]
  fn message_truncates_to_the_first_line() {
    assert_eq!(truncate_message("fix: thing\n\nbody", 60), "fix: thing");
    assert_eq!(truncate_message("abcdefghij", 5), "abcd…");
  }
}
