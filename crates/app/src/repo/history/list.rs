use std::sync::Arc;

use chrono::Utc;
use deathpush_core::ops::history::{RESET_MODES, commit_id_menu_label};
use deathpush_core::relative_time::relative_time;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::CommitEntry;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::menu::{ContextMenuExt, PopupMenu, PopupMenuItem};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::avatar::render_avatar;
use super::view::HistoryView;
use crate::theme::hsla;

pub fn render_list(
  log: Arc<Vec<CommitEntry>>,
  selected: Option<&str>,
  file_history_path: Option<&str>,
  has_more: bool,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> impl IntoElement {
  let empty = log.is_empty();
  div()
    .size_full()
    .flex()
    .flex_col()
    .bg(hsla(palette.sidebar))
    .border_r_1()
    .border_color(hsla(palette.border))
    .children(file_history_path.map(|path| render_chip(path, view.clone(), palette)))
    .when(empty, |el| el.child(render_empty(palette)))
    .when(!empty, |el| {
      let count = log.len();
      let selected = selected.map(str::to_string);
      let list_log = log.clone();
      let list_view = view.clone();
      let list = uniform_list("history-log", count, move |range, _, _| {
        range
          .filter_map(|index| {
            let entry = list_log.get(index)?;
            Some(
              render_commit_row(
                entry,
                selected.as_deref() == Some(entry.id.as_str()),
                list_view.clone(),
                palette,
              )
              .into_any_element(),
            )
          })
          .collect()
      });
      el.child(list.flex_1().min_h_0())
    })
    .when(has_more, |el| {
      let load = view.clone();
      el.child(
        div().flex_shrink_0().p_2().child(
          Button::new("history-load-more")
            .outline()
            .small()
            .label("Load More")
            .on_click(move |_, window, cx| {
              let _ = load.update(cx, |this, cx| this.load_more(window, cx));
            }),
        ),
      )
    })
}

fn render_chip(path: &str, view: WeakEntity<HistoryView>, palette: UiPalette) -> impl IntoElement {
  let name = path.rsplit(['/', '\\']).next().unwrap_or(path).to_string();
  div()
    .h(px(28.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_2()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      svg()
        .path("icons/history.svg")
        .size(px(14.0))
        .flex_shrink_0()
        .text_color(hsla(palette.muted_foreground)),
    )
    .child(
      div()
        .flex_1()
        .min_w_0()
        .truncate()
        .text_size(px(12.0))
        .text_color(hsla(palette.foreground))
        .child(name),
    )
    .child(
      Button::new("history-clear-file")
        .ghost()
        .xsmall()
        .w(px(22.0))
        .h(px(22.0))
        .icon(Icon::empty().path("icons/close.svg"))
        .tooltip("Show full history")
        .on_click(move |_, window, cx| {
          let _ = view.update(cx, |this, cx| this.clear_file_history(window, cx));
        }),
    )
}

fn render_empty(palette: UiPalette) -> impl IntoElement {
  div()
    .flex_1()
    .min_h_0()
    .flex()
    .items_center()
    .justify_center()
    .text_size(px(13.0))
    .text_color(hsla(palette.muted_foreground))
    .child("No commits found")
}

fn render_commit_row(
  entry: &CommitEntry,
  selected: bool,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> impl IntoElement {
  let id = entry.id.clone();
  let click_id = id.clone();
  let click_view = view.clone();
  let subject = subject_line(&entry.message).to_string();
  let when = relative_time(&entry.author_date, Utc::now());
  let merge = entry.parent_ids.len() > 1;
  div()
    .id(SharedString::from(format!("history-commit-{id}")))
    .h(px(44.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_2()
    .px_3()
    .border_b_1()
    .border_color(hsla(palette.border))
    .cursor_pointer()
    .when(selected, |el| el.bg(hsla(palette.list_active)))
    .when(!selected, |el| el.hover(|el| el.bg(hsla(palette.list_hover))))
    .on_click(move |_, window, cx| {
      let _ = click_view.update(cx, |this, cx| this.select_commit(click_id.clone(), window, cx));
    })
    .context_menu({
      let view = view.clone();
      let entry = entry.clone();
      move |menu, _, _| fill_commit_menu(menu, &entry, view.clone())
    })
    .child(render_avatar(entry, &palette))
    .child(
      div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .child(
          div()
            .flex()
            .items_center()
            .justify_between()
            .gap_2()
            .child(
              div()
                .min_w_0()
                .flex_1()
                .truncate()
                .text_size(px(13.0))
                .text_color(hsla(palette.foreground))
                .child(subject),
            )
            .child(
              div()
                .flex_shrink_0()
                .text_size(px(11.0))
                .text_color(hsla(palette.muted_foreground))
                .child(when),
            ),
        )
        .child(
          div()
            .flex()
            .items_center()
            .gap_2()
            .child(
              div()
                .flex_shrink_0()
                .text_size(px(11.0))
                .text_color(hsla(palette.muted_foreground))
                .child(entry.short_id.clone()),
            )
            .when(merge, |el| {
              el.child(
                div()
                  .px_1()
                  .rounded_full()
                  .text_size(px(10.0))
                  .bg(hsla(palette.badge))
                  .text_color(hsla(palette.badge_foreground))
                  .child("merge"),
              )
            })
            .child(
              div()
                .min_w_0()
                .truncate()
                .text_size(px(11.0))
                .text_color(hsla(palette.muted_foreground))
                .child(entry.author_name.clone()),
            ),
        ),
    )
}

fn fill_commit_menu(menu: PopupMenu, entry: &CommitEntry, view: WeakEntity<HistoryView>) -> PopupMenu {
  let mut menu = menu.min_w(px(180.));
  let copy_id = entry.id.clone();
  let copy_id_view = view.clone();
  menu = menu.item(
    PopupMenuItem::new(commit_id_menu_label(&entry.short_id)).on_click(move |_, _, cx| {
      let _ = copy_id_view.update(cx, |this, cx| this.copy(copy_id.clone(), cx));
    }),
  );
  let copy_message = entry.message.clone();
  let copy_message_view = view.clone();
  menu = menu.item(PopupMenuItem::new("Copy Commit Message").on_click(move |_, _, cx| {
    let _ = copy_message_view.update(cx, |this, cx| this.copy(copy_message.clone(), cx));
  }));
  let cherry = entry.id.clone();
  let cherry_view = view.clone();
  menu = menu.item(PopupMenuItem::new("Cherry-pick Commit").on_click(move |_, window, cx| {
    let _ = cherry_view.update(cx, |this, cx| this.cherry_pick(cherry.clone(), window, cx));
  }));
  for &(label, mode) in &RESET_MODES {
    let view = view.clone();
    let commit = entry.id.clone();
    menu = menu.item(PopupMenuItem::new(label).on_click(move |_, window, cx| {
      let _ = view.update(cx, |this, cx| this.reset(commit.clone(), mode.to_string(), window, cx));
    }));
  }
  menu
}

pub fn subject_line(message: &str) -> &str {
  message.lines().next().unwrap_or("").trim()
}
