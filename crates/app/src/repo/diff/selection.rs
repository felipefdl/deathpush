use std::ops::Range;

use deathpush_core::diff_view::{DiffRow, DiffRows, RowKind};

use super::highlight::Side;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Anchor {
  pub row: usize,
  pub side: Side,
  pub byte: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
  pub anchor: Anchor,
  pub head: Anchor,
}

impl Selection {
  pub fn ordered(&self) -> (Anchor, Anchor) {
    if self.anchor <= self.head {
      (self.anchor, self.head)
    } else {
      (self.head, self.anchor)
    }
  }

  pub fn is_empty(&self) -> bool {
    self.anchor == self.head
  }

  /// Byte range selected inside `row` on `side`, if any.
  /// Side-by-side rows only select on the anchor side; inline rows accept either side.
  pub fn range_in(&self, row: usize, side: Side, len: usize, side_by_side: bool) -> Option<Range<usize>> {
    if self.is_empty() {
      return None;
    }
    let (start, end) = self.ordered();
    if row < start.row || row > end.row {
      return None;
    }
    if side_by_side && side != self.anchor.side {
      return None;
    }
    let range = if start.row == end.row {
      let a = start.byte.min(end.byte).min(len);
      let b = start.byte.max(end.byte).min(len);
      a..b
    } else if row == start.row {
      start.byte.min(len)..len
    } else if row == end.row {
      0..end.byte.min(len)
    } else {
      0..len
    };
    if range.start >= range.end { None } else { Some(range) }
  }

  /// The selected text, rows joined with "\n"; separator and empty rows are skipped; side-by-side copies the side the anchor is on.
  pub fn text(&self, rows: &DiffRows) -> String {
    if self.is_empty() {
      return String::new();
    }
    let (start, end) = self.ordered();
    let mut parts = Vec::new();
    match rows {
      DiffRows::Inline(items) => {
        for i in start.row..=end.row {
          let Some(row) = items.get(i) else {
            break;
          };
          if skip_row(row) {
            continue;
          }
          if let Some(range) = self.range_in(i, start.side, row.text.len(), false)
            && let Some(slice) = row.text.get(range)
          {
            parts.push(slice);
          }
        }
      }
      DiffRows::SideBySide(items) => {
        let side = self.anchor.side;
        for i in start.row..=end.row {
          let Some(pair) = items.get(i) else {
            break;
          };
          let row = match side {
            Side::Old => &pair.left,
            Side::New => &pair.right,
          };
          if skip_row(row) {
            continue;
          }
          if let Some(range) = self.range_in(i, side, row.text.len(), true)
            && let Some(slice) = row.text.get(range)
          {
            parts.push(slice);
          }
        }
      }
    }
    parts.join("\n")
  }
}

fn skip_row(row: &DiffRow) -> bool {
  matches!(row.kind, RowKind::Separator | RowKind::Empty)
}

pub fn row_at(rows: &DiffRows, index: usize, side: Side) -> Option<&DiffRow> {
  match rows {
    DiffRows::Inline(items) => items.get(index),
    DiffRows::SideBySide(items) => items.get(index).map(|pair| match side {
      Side::Old => &pair.left,
      Side::New => &pair.right,
    }),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::diff_view::SideRow;

  fn rows() -> DiffRows {
    DiffRows::Inline(vec![
      DiffRow {
        kind: RowKind::Separator,
        hunk: 0,
        old_line: None,
        new_line: None,
        text: "...".into(),
        changed: vec![],
      },
      DiffRow {
        kind: RowKind::Context,
        hunk: 0,
        old_line: Some(1),
        new_line: Some(1),
        text: "alpha".into(),
        changed: vec![],
      },
      DiffRow {
        kind: RowKind::Remove,
        hunk: 0,
        old_line: Some(2),
        new_line: None,
        text: "beta".into(),
        changed: vec![],
      },
      DiffRow {
        kind: RowKind::Add,
        hunk: 0,
        old_line: None,
        new_line: Some(2),
        text: "gamma".into(),
        changed: vec![],
      },
    ])
  }

  #[test]
  fn text_spans_rows_and_skips_separators() {
    let sel = Selection {
      anchor: Anchor {
        row: 3,
        side: Side::New,
        byte: 2,
      },
      head: Anchor {
        row: 0,
        side: Side::New,
        byte: 1,
      },
    };
    assert_eq!(sel.text(&rows()), "alpha\nbeta\nga");
    assert_eq!(sel.range_in(1, Side::New, 5, false), Some(0..5));
    assert_eq!(sel.range_in(3, Side::New, 5, false), Some(0..2));
    assert_eq!(
      sel.range_in(2, Side::Old, 4, false),
      Some(0..4),
      "inline rows accept either side"
    );
    assert!(sel.range_in(4, Side::New, 1, false).is_none());
  }

  #[test]
  fn single_row_range_and_empty() {
    let sel = Selection {
      anchor: Anchor {
        row: 1,
        side: Side::New,
        byte: 1,
      },
      head: Anchor {
        row: 1,
        side: Side::New,
        byte: 4,
      },
    };
    assert_eq!(sel.range_in(1, Side::New, 5, false), Some(1..4));
    assert_eq!(sel.text(&rows()), "lph");
    let empty = Selection {
      anchor: sel.anchor,
      head: sel.anchor,
    };
    assert!(empty.is_empty());
  }

  fn side_row(kind: RowKind, text: &str, old: Option<usize>, new: Option<usize>) -> DiffRow {
    DiffRow {
      kind,
      hunk: 0,
      old_line: old,
      new_line: new,
      text: text.into(),
      changed: vec![],
    }
  }

  #[test]
  fn side_by_side_copies_the_anchor_side_only() {
    let rows = DiffRows::SideBySide(vec![
      SideRow {
        left: side_row(RowKind::Remove, "old", Some(1), None),
        right: side_row(RowKind::Add, "new", None, Some(1)),
      },
      SideRow {
        left: side_row(RowKind::Remove, "foo", Some(2), None),
        right: side_row(RowKind::Add, "bar", None, Some(2)),
      },
    ]);
    let sel = Selection {
      anchor: Anchor {
        row: 0,
        side: Side::Old,
        byte: 0,
      },
      head: Anchor {
        row: 1,
        side: Side::Old,
        byte: 3,
      },
    };
    assert_eq!(sel.text(&rows), "old\nfoo");
    assert!(sel.range_in(0, Side::New, 3, true).is_none());
    assert!(sel.range_in(1, Side::New, 3, true).is_none());
    assert_eq!(sel.range_in(0, Side::Old, 3, true), Some(0..3));
    assert_eq!(sel.range_in(1, Side::Old, 3, true), Some(0..3));
  }
}
