//! Rows for the diff view: the `scm_file_diff` payload plus the diff settings in, painted rows out.
//! Pure so its tests run without a window.

use std::ops::Range;

use similar::{ChangeTag, TextDiff};

use crate::config::settings::{DiffLayout, HunkSeparators, LineDiffType};
use crate::session::types::{DiffHunkPayload, DiffPayload};
use crate::types::DiffLine;

pub struct RowOptions {
  pub layout: DiffLayout,
  pub line_diff: LineDiffType,
  pub separators: HunkSeparators,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowKind {
  Context,
  Add,
  Remove,
  /// Hunk header row; `hunk` indexes `DiffPayload::hunks`.
  Separator,
  /// Side-by-side filler opposite a one-sided change.
  Empty,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffRow {
  pub kind: RowKind,
  pub hunk: usize,
  pub old_line: Option<usize>,
  pub new_line: Option<usize>,
  /// Line text without the trailing newline; the separator label for `Separator`.
  pub text: String,
  /// Byte ranges inside `text` that differ from the paired line (word, alt-word, or char granularity).
  pub changed: Vec<std::ops::Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SideRow {
  pub left: DiffRow,
  pub right: DiffRow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffRows {
  Inline(Vec<DiffRow>),
  SideBySide(Vec<SideRow>),
}

pub fn separator_label(hunk: &DiffHunkPayload, separators: HunkSeparators) -> String {
  let old_end = hunk.old_start + hunk.old_lines.saturating_sub(1);
  let new_end = hunk.new_start + hunk.new_lines.saturating_sub(1);
  match separators {
    HunkSeparators::Simple => "...".to_string(),
    HunkSeparators::Metadata => hunk.header.clone(),
    HunkSeparators::LineInfo => {
      format!(
        "Lines {}-{} (old) / {}-{} (new)",
        hunk.old_start, old_end, hunk.new_start, new_end
      )
    }
    HunkSeparators::LineInfoBasic => format!("{}-{} / {}-{}", hunk.old_start, old_end, hunk.new_start, new_end),
  }
}

fn text_of(line: &DiffLine) -> String {
  line.content.trim_end_matches(['\n', '\r']).to_string()
}

fn separator_row(hunk_index: usize, hunk: &DiffHunkPayload, separators: HunkSeparators) -> DiffRow {
  DiffRow {
    kind: RowKind::Separator,
    hunk: hunk_index,
    old_line: None,
    new_line: None,
    text: separator_label(hunk, separators),
    changed: Vec::new(),
  }
}

fn empty_row(hunk_index: usize) -> DiffRow {
  DiffRow {
    kind: RowKind::Empty,
    hunk: hunk_index,
    old_line: None,
    new_line: None,
    text: String::new(),
    changed: Vec::new(),
  }
}

fn row_for(line: &DiffLine, hunk_index: usize) -> DiffRow {
  let kind = match line.line_type.as_str() {
    "add" => RowKind::Add,
    "remove" => RowKind::Remove,
    _ => RowKind::Context,
  };
  DiffRow {
    kind,
    hunk: hunk_index,
    old_line: line.old_line_number,
    new_line: line.new_line_number,
    text: text_of(line),
    changed: Vec::new(),
  }
}

/// Byte ranges of `side` that differ from `other`, at the requested granularity.
/// `WordAlt` is word granularity that gives up (no ranges) when fewer than half of the words match,
/// so a rewritten line is painted as a whole line instead of a wall of marks.
pub fn word_ranges(side: &str, other: &str, line_diff: LineDiffType, side_is_old: bool) -> Vec<Range<usize>> {
  let diff = match line_diff {
    LineDiffType::None => return Vec::new(),
    LineDiffType::Char => TextDiff::from_chars(
      if side_is_old { side } else { other },
      if side_is_old { other } else { side },
    ),
    LineDiffType::Word | LineDiffType::WordAlt => TextDiff::from_unicode_words(
      if side_is_old { side } else { other },
      if side_is_old { other } else { side },
    ),
  };
  let wanted = if side_is_old {
    ChangeTag::Delete
  } else {
    ChangeTag::Insert
  };
  let mut ranges: Vec<Range<usize>> = Vec::new();
  let mut offset = 0usize;
  let mut equal_words = 0usize;
  let mut side_words = 0usize;
  for change in diff.iter_all_changes() {
    let text = change.value();
    let on_this_side = match change.tag() {
      ChangeTag::Equal => true,
      tag => tag == wanted,
    };
    if !on_this_side {
      continue;
    }
    if !text.trim().is_empty() {
      side_words += 1;
      if change.tag() == ChangeTag::Equal {
        equal_words += 1;
      }
    }
    if change.tag() == wanted {
      let start = offset;
      let end = offset + text.len();
      match ranges.last_mut() {
        Some(last) if last.end == start => last.end = end,
        _ => ranges.push(start..end),
      }
    }
    offset += text.len();
  }
  if line_diff == LineDiffType::WordAlt && side_words > 0 && equal_words * 2 < side_words {
    return Vec::new();
  }
  ranges
}

/// Pairs the k-th remove with the k-th add inside one change block, the way editors paint modified lines.
/// Returns (removes, adds) for the block starting at `start`, and the index after the block.
fn change_block(lines: &[DiffLine], start: usize) -> (Vec<usize>, Vec<usize>, usize) {
  let mut removes = Vec::new();
  let mut adds = Vec::new();
  let mut i = start;
  while i < lines.len() && lines[i].line_type == "remove" {
    removes.push(i);
    i += 1;
  }
  while i < lines.len() && lines[i].line_type == "add" {
    adds.push(i);
    i += 1;
  }
  (removes, adds, i)
}

pub fn build_rows(payload: &DiffPayload, options: &RowOptions) -> DiffRows {
  match options.layout {
    DiffLayout::Inline => DiffRows::Inline(build_inline(payload, options)),
    DiffLayout::SideBySide => DiffRows::SideBySide(build_side_by_side(payload, options)),
  }
}

fn build_inline(payload: &DiffPayload, options: &RowOptions) -> Vec<DiffRow> {
  let mut rows = Vec::new();
  for (hunk_index, hunk) in payload.hunks.iter().enumerate() {
    rows.push(separator_row(hunk_index, hunk, options.separators));
    let lines = &hunk.lines;
    let mut i = 0;
    while i < lines.len() {
      if lines[i].line_type == "context" {
        rows.push(row_for(&lines[i], hunk_index));
        i += 1;
        continue;
      }
      let (removes, adds, next) = change_block(lines, i);
      for (k, &r) in removes.iter().enumerate() {
        let mut row = row_for(&lines[r], hunk_index);
        if let Some(&a) = adds.get(k) {
          row.changed = word_ranges(&row.text, &text_of(&lines[a]), options.line_diff, true);
        }
        rows.push(row);
      }
      for (k, &a) in adds.iter().enumerate() {
        let mut row = row_for(&lines[a], hunk_index);
        if let Some(&r) = removes.get(k) {
          row.changed = word_ranges(&row.text, &text_of(&lines[r]), options.line_diff, false);
        }
        rows.push(row);
      }
      i = next;
    }
  }
  rows
}

fn build_side_by_side(payload: &DiffPayload, options: &RowOptions) -> Vec<SideRow> {
  let mut rows = Vec::new();
  for (hunk_index, hunk) in payload.hunks.iter().enumerate() {
    rows.push(SideRow {
      left: separator_row(hunk_index, hunk, options.separators),
      right: separator_row(hunk_index, hunk, options.separators),
    });
    let lines = &hunk.lines;
    let mut i = 0;
    while i < lines.len() {
      if lines[i].line_type == "context" {
        let row = row_for(&lines[i], hunk_index);
        rows.push(SideRow {
          left: row.clone(),
          right: row,
        });
        i += 1;
        continue;
      }
      let (removes, adds, next) = change_block(lines, i);
      for k in 0..removes.len().max(adds.len()) {
        let left = match removes.get(k) {
          Some(&r) => {
            let mut row = row_for(&lines[r], hunk_index);
            if let Some(&a) = adds.get(k) {
              row.changed = word_ranges(&row.text, &text_of(&lines[a]), options.line_diff, true);
            }
            row
          }
          None => empty_row(hunk_index),
        };
        let right = match adds.get(k) {
          Some(&a) => {
            let mut row = row_for(&lines[a], hunk_index);
            if let Some(&r) = removes.get(k) {
              row.changed = word_ranges(&row.text, &text_of(&lines[r]), options.line_diff, false);
            }
            row
          }
          None => empty_row(hunk_index),
        };
        rows.push(SideRow { left, right });
      }
      i = next;
    }
  }
  rows
}

impl DiffRows {
  pub fn len(&self) -> usize {
    match self {
      DiffRows::Inline(rows) => rows.len(),
      DiffRows::SideBySide(rows) => rows.len(),
    }
  }

  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  pub fn max_columns(&self) -> usize {
    match self {
      DiffRows::Inline(rows) => rows.iter().map(|row| row.text.chars().count()).max().unwrap_or(0),
      DiffRows::SideBySide(rows) => rows
        .iter()
        .map(|row| row.left.text.chars().count().max(row.right.text.chars().count()))
        .max()
        .unwrap_or(0),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::session::types::{DiffHunkPayload, DiffPayload, DiffPresence};
  use crate::types::DiffLine;

  fn line(kind: &str, content: &str, old: Option<usize>, new: Option<usize>) -> DiffLine {
    DiffLine {
      content: content.to_string(),
      line_type: kind.to_string(),
      old_line_number: old,
      new_line_number: new,
    }
  }

  fn payload(hunks: Vec<DiffHunkPayload>) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      original: String::new(),
      modified: String::new(),
      language: Some("rust".into()),
      file_type: "text".into(),
      hunks,
      presence: DiffPresence {
        old_exists: true,
        new_exists: true,
      },
      editable: true,
      enable_line_selection: true,
      staged: false,
      content_hash: "h".into(),
    }
  }

  fn hunk(lines: Vec<DiffLine>) -> DiffHunkPayload {
    DiffHunkPayload {
      id: "abc".into(),
      header: "@@ -1,3 +1,3 @@ fn main".into(),
      old_start: 1,
      old_lines: 3,
      new_start: 1,
      new_lines: 3,
      lines,
    }
  }

  fn options(layout: DiffLayout, line_diff: LineDiffType) -> RowOptions {
    RowOptions {
      layout,
      line_diff,
      separators: HunkSeparators::Simple,
    }
  }

  #[test]
  fn inline_rows_start_each_hunk_with_a_separator() {
    let p = payload(vec![hunk(vec![
      line("context", "fn main() {", Some(1), Some(1)),
      line("remove", "  let a = 1;", Some(2), None),
      line("add", "  let a = 2;", None, Some(2)),
      line("context", "}", Some(3), Some(3)),
    ])]);
    let DiffRows::Inline(rows) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::None)) else {
      panic!("inline expected");
    };
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].kind, RowKind::Separator);
    assert_eq!(rows[0].text, "...");
    assert_eq!(rows[1].kind, RowKind::Context);
    assert_eq!(
      (rows[2].kind, rows[2].old_line, rows[2].new_line),
      (RowKind::Remove, Some(2), None)
    );
    assert_eq!(
      (rows[3].kind, rows[3].old_line, rows[3].new_line),
      (RowKind::Add, None, Some(2))
    );
    assert!(rows.iter().all(|row| row.hunk == 0));
  }

  #[test]
  fn side_by_side_pairs_removes_with_adds_and_fills_the_rest() {
    let p = payload(vec![hunk(vec![
      line("context", "a", Some(1), Some(1)),
      line("remove", "b", Some(2), None),
      line("remove", "c", Some(3), None),
      line("add", "B", None, Some(2)),
      line("context", "d", Some(4), Some(3)),
    ])]);
    let DiffRows::SideBySide(rows) = build_rows(&p, &options(DiffLayout::SideBySide, LineDiffType::None)) else {
      panic!("side by side expected");
    };
    // separator, context, (b|B), (c|empty), context
    assert_eq!(rows.len(), 5);
    assert_eq!(rows[0].left.kind, RowKind::Separator);
    assert_eq!(rows[0].right.kind, RowKind::Separator);
    assert_eq!((rows[2].left.kind, rows[2].left.text.as_str()), (RowKind::Remove, "b"));
    assert_eq!((rows[2].right.kind, rows[2].right.text.as_str()), (RowKind::Add, "B"));
    assert_eq!((rows[3].left.kind, rows[3].left.text.as_str()), (RowKind::Remove, "c"));
    assert_eq!(rows[3].right.kind, RowKind::Empty);
    assert_eq!((rows[4].left.old_line, rows[4].right.new_line), (Some(4), Some(3)));
  }

  #[test]
  fn word_ranges_mark_only_the_changed_words() {
    let p = payload(vec![hunk(vec![
      line("remove", "let value = 1;", Some(1), None),
      line("add", "let value = 2;", None, Some(1)),
    ])]);
    let DiffRows::Inline(rows) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::Word)) else {
      panic!("inline expected");
    };
    assert_eq!(rows[1].changed, vec![12..13]);
    assert_eq!(rows[2].changed, vec![12..13]);
  }

  #[test]
  fn char_ranges_and_none() {
    let p = payload(vec![hunk(vec![
      line("remove", "abcd", Some(1), None),
      line("add", "abXd", None, Some(1)),
    ])]);
    let DiffRows::Inline(chars) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::Char)) else {
      panic!("inline expected");
    };
    assert_eq!(chars[1].changed, vec![2..3]);
    assert_eq!(chars[2].changed, vec![2..3]);
    let DiffRows::Inline(none) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::None)) else {
      panic!("inline expected");
    };
    assert!(none[1].changed.is_empty() && none[2].changed.is_empty());
  }

  #[test]
  fn word_alt_skips_ranges_when_the_lines_share_nothing() {
    let p = payload(vec![hunk(vec![
      line("remove", "alpha beta", Some(1), None),
      line("add", "gamma delta", None, Some(1)),
    ])]);
    let DiffRows::Inline(rows) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::WordAlt)) else {
      panic!("inline expected");
    };
    assert!(rows[1].changed.is_empty());
    assert!(rows[2].changed.is_empty());
    let DiffRows::Inline(word) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::Word)) else {
      panic!("inline expected");
    };
    // observed tokens: "alpha" / " " / "beta" (space Equal), so two ranges
    assert_eq!(word[1].changed, vec![0..5, 6..10]);
  }

  #[test]
  fn separator_labels_follow_the_setting() {
    let h = hunk(vec![]);
    assert_eq!(separator_label(&h, HunkSeparators::Simple), "...");
    assert_eq!(separator_label(&h, HunkSeparators::Metadata), "@@ -1,3 +1,3 @@ fn main");
    assert_eq!(
      separator_label(&h, HunkSeparators::LineInfo),
      "Lines 1-3 (old) / 1-3 (new)"
    );
    assert_eq!(separator_label(&h, HunkSeparators::LineInfoBasic), "1-3 / 1-3");
  }

  #[test]
  fn max_columns_counts_characters_on_both_sides() {
    let p = payload(vec![hunk(vec![
      line("remove", "héllo wörld", Some(1), None),
      line("add", "hi", None, Some(1)),
    ])]);
    let rows = build_rows(&p, &options(DiffLayout::SideBySide, LineDiffType::None));
    assert_eq!(rows.max_columns(), 11);
    assert_eq!(rows.len(), 2);
    assert!(!rows.is_empty());
  }

  #[test]
  fn trailing_newlines_are_stripped_from_row_text() {
    let p = payload(vec![hunk(vec![line("context", "abc\n", Some(1), Some(1))])]);
    let DiffRows::Inline(rows) = build_rows(&p, &options(DiffLayout::Inline, LineDiffType::None)) else {
      panic!("inline expected");
    };
    assert_eq!(rows[1].text, "abc");
  }
}
