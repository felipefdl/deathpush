use chrono::Utc;
use deathpush_core::ops::history::{FileNode, changed_files_tree, merge_parents_label};
use deathpush_core::relative_time::relative_time;
use deathpush_core::theme::UiPalette;
use deathpush_core::types::{CommitEntry, CommitFileEntry};
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::FluentBuilder;
use gpui_kit::*;

use super::list::subject_line;
use super::view::HistoryView;
use crate::repo::changes::rows::{status_color, status_letter};
use crate::repo::explorer::icons::{IconKind, icon_for};
use crate::theme::hsla;

pub fn file_label(file: &CommitFileEntry) -> String {
  match &file.old_path {
    Some(old) if !old.is_empty() => format!("{old} -> {}", file.path),
    _ => file.path.clone(),
  }
}

pub fn render_empty(palette: UiPalette) -> impl IntoElement {
  div()
    .size_full()
    .flex()
    .flex_col()
    .items_center()
    .justify_center()
    .gap_2()
    .child(
      svg()
        .path("icons/history.svg")
        .size(px(48.0))
        .text_color(hsla(palette.muted_foreground))
        .opacity(0.4),
    )
    .child(
      div()
        .text_size(px(13.0))
        .text_color(hsla(palette.muted_foreground))
        .child("Select a commit to view details"),
    )
}

pub fn render_header(entry: &CommitEntry, view: WeakEntity<HistoryView>, palette: UiPalette) -> impl IntoElement {
  let subject = subject_line(&entry.message).to_string();
  let body = message_body(&entry.message).map(str::to_string);
  let when = relative_time(&entry.author_date, Utc::now());
  let merge = merge_parents_label(entry);
  let sha = entry.id.clone();
  let message = entry.message.clone();
  let email = entry.author_email.clone();
  div()
    .flex_shrink_0()
    .flex()
    .flex_col()
    .px_3()
    .py_2()
    .gap_1()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      div()
        .flex()
        .items_center()
        .gap_2()
        .child(
          div()
            .min_w_0()
            .flex_1()
            .truncate()
            .text_size(px(14.0))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(hsla(palette.foreground))
            .child(subject),
        )
        .child(
          div()
            .flex()
            .items_center()
            .gap_1()
            .flex_shrink_0()
            .child(copy_button("history-copy-sha", "Copy full SHA", sha, view.clone()))
            .child(copy_button(
              "history-copy-message",
              "Copy commit message",
              message,
              view.clone(),
            ))
            .child(copy_button("history-copy-email", "Copy email", email, view)),
        ),
    )
    .child(
      div()
        .flex()
        .items_center()
        .gap_1()
        .min_w_0()
        .child(
          div()
            .flex_shrink_0()
            .text_size(px(12.0))
            .text_color(hsla(palette.accent))
            .child(entry.short_id.clone()),
        )
        .child(middot(palette))
        .child(
          div()
            .min_w_0()
            .truncate()
            .text_size(px(12.0))
            .text_color(hsla(palette.foreground))
            .child(entry.author_name.clone()),
        )
        .child(middot(palette))
        .child(
          div()
            .flex_shrink_0()
            .text_size(px(12.0))
            .text_color(hsla(palette.muted_foreground))
            .child(when),
        ),
    )
    .when_some(body, |el, body| {
      el.child(
        div()
          .text_size(px(12.0))
          .text_color(hsla(palette.muted_foreground))
          .child(body),
      )
    })
    .when_some(merge, |el, label| {
      el.child(
        div()
          .text_size(px(12.0))
          .text_color(hsla(palette.muted_foreground))
          .child(label),
      )
    })
}

pub fn render_files(
  files: &[CommitFileEntry],
  as_tree: bool,
  selected_path: Option<&str>,
  commit: &str,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> impl IntoElement {
  let n = files.len();
  let (icon, tooltip) = if as_tree {
    ("icons/list-flat.svg", "Show as list")
  } else {
    ("icons/list-tree.svg", "Show as tree")
  };
  let toggle = view.clone();
  div()
    .flex_shrink_0()
    .flex()
    .flex_col()
    .border_b_1()
    .border_color(hsla(palette.border))
    .child(
      div()
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_between()
        .px_3()
        .child(
          div()
            .text_size(px(11.0))
            .font_weight(FontWeight::BOLD)
            .text_color(hsla(palette.muted_foreground))
            .child(format!("Changed Files ({n})").to_uppercase()),
        )
        .child(
          Button::new("history-files-toggle")
            .ghost()
            .xsmall()
            .w(px(22.0))
            .h(px(22.0))
            .icon(Icon::empty().path(icon))
            .tooltip(tooltip)
            .on_click(move |_, _, cx| {
              let _ = toggle.update(cx, |this, cx| this.toggle_files_as_tree(cx));
            }),
        ),
    )
    .child(
      div()
        .id("history-files")
        .max_h(px(176.0))
        .overflow_y_scroll()
        .flex()
        .flex_col()
        .children(file_rows(files, as_tree, selected_path, commit, view, palette)),
    )
}

fn file_rows(
  files: &[CommitFileEntry],
  as_tree: bool,
  selected_path: Option<&str>,
  commit: &str,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> Vec<AnyElement> {
  if as_tree {
    flatten_tree(&changed_files_tree(files), 0)
      .into_iter()
      .map(|(depth, node)| render_tree_row(&node, depth, selected_path, commit, view.clone(), palette))
      .collect()
  } else {
    files
      .iter()
      .map(|file| render_file_row(file, 0, selected_path, commit, view.clone(), palette))
      .collect()
  }
}

fn flatten_tree(nodes: &[FileNode], depth: usize) -> Vec<(usize, FileNode)> {
  let mut out = Vec::new();
  for node in nodes {
    out.push((depth, node.clone()));
    out.extend(flatten_tree(&node.children, depth + 1));
  }
  out
}

fn render_tree_row(
  node: &FileNode,
  depth: usize,
  selected_path: Option<&str>,
  commit: &str,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> AnyElement {
  if let Some(file) = &node.file {
    render_file_row(file, depth, selected_path, commit, view, palette)
  } else {
    let icon = icon_for(IconKind::Standard, &node.name, true, true);
    div()
      .id(SharedString::from(format!("history-folder-{}", node.path)))
      .h(px(22.0))
      .flex_shrink_0()
      .flex()
      .items_center()
      .gap_1()
      .px_3()
      .pl(px(12.0 + 12.0 * depth as f32))
      .when_some(icon, |el, path| {
        el.child(
          svg()
            .path(path)
            .size(px(16.0))
            .flex_shrink_0()
            .text_color(hsla(palette.muted_foreground)),
        )
      })
      .child(
        div()
          .min_w_0()
          .flex_1()
          .truncate()
          .text_size(px(13.0))
          .text_color(hsla(palette.foreground))
          .child(node.name.clone()),
      )
      .into_any_element()
  }
}

fn render_file_row(
  file: &CommitFileEntry,
  depth: usize,
  selected_path: Option<&str>,
  commit: &str,
  view: WeakEntity<HistoryView>,
  palette: UiPalette,
) -> AnyElement {
  let selected = selected_path == Some(file.path.as_str());
  let label = if depth == 0 {
    file_label(file)
  } else {
    file.path.rsplit('/').next().unwrap_or(&file.path).to_string()
  };
  let name = file.path.rsplit('/').next().unwrap_or(&file.path);
  let icon = icon_for(IconKind::Standard, name, false, false);
  let letter = status_letter(file.status.clone());
  let color = status_color(file.status.clone(), &palette);
  let commit = commit.to_string();
  let path = file.path.clone();
  let status = file.status.clone();
  div()
    .id(SharedString::from(format!("history-file-{}", file.path)))
    .h(px(22.0))
    .flex_shrink_0()
    .flex()
    .items_center()
    .gap_1()
    .px_3()
    .pl(px(12.0 + 12.0 * depth as f32))
    .cursor_pointer()
    .when(selected, |el| el.bg(hsla(palette.list_active)))
    .when(!selected, |el| el.hover(|el| el.bg(hsla(palette.list_hover))))
    .on_click(move |_, window, cx| {
      let _ = view.update(cx, |this, cx| {
        this.open_commit_file(commit.clone(), path.clone(), status.clone(), window, cx);
      });
    })
    .when_some(icon, |el, path| {
      el.child(
        svg()
          .path(path)
          .size(px(16.0))
          .flex_shrink_0()
          .text_color(hsla(palette.muted_foreground)),
      )
    })
    .child(
      div()
        .min_w_0()
        .flex_1()
        .truncate()
        .text_size(px(13.0))
        .text_color(hsla(palette.foreground))
        .child(label),
    )
    .child(
      div()
        .w(px(16.0))
        .flex_shrink_0()
        .text_size(px(11.0))
        .text_color(hsla(color))
        .child(letter),
    )
    .into_any_element()
}

fn copy_button(
  id: &'static str,
  tooltip: &'static str,
  text: String,
  view: WeakEntity<HistoryView>,
) -> impl IntoElement {
  Button::new(id)
    .ghost()
    .xsmall()
    .w(px(22.0))
    .h(px(22.0))
    .icon(Icon::empty().path("icons/copy.svg"))
    .tooltip(tooltip)
    .on_click(move |_, _, cx| {
      let _ = view.update(cx, |this, cx| this.copy(text.clone(), cx));
    })
}

fn middot(palette: UiPalette) -> impl IntoElement {
  div()
    .flex_shrink_0()
    .text_size(px(12.0))
    .text_color(hsla(palette.muted_foreground))
    .child(" · ")
}

fn message_body(message: &str) -> Option<&str> {
  let rest = message.split_once('\n')?.1.trim();
  if rest.is_empty() { None } else { Some(rest) }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::types::FileStatus;

  #[test]
  fn file_label_uses_rename_arrow() {
    let renamed = CommitFileEntry {
      path: "b.rs".into(),
      status: FileStatus::Renamed,
      old_path: Some("a.rs".into()),
    };
    assert_eq!(file_label(&renamed), "a.rs -> b.rs");
    let plain = CommitFileEntry {
      path: "b.rs".into(),
      status: FileStatus::Modified,
      old_path: None,
    };
    assert_eq!(file_label(&plain), "b.rs");
  }
}
