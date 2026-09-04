//! Pure terminal snapshot types. Send + Sync; no libghostty types.

/// An RGB color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb(
  /// Red, 0-255.
  pub u8,
  /// Green, 0-255.
  pub u8,
  /// Blue, 0-255.
  pub u8,
);

/// One cell in a pane snapshot.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnapshotCell {
  /// Cell text; a single space when empty.
  pub text: String,
  /// Foreground color, falling back to the snapshot default.
  pub fg: Option<Rgb>,
  /// Background color, falling back to the snapshot default.
  pub bg: Option<Rgb>,
  /// Bold.
  pub bold: bool,
  /// Italic.
  pub italic: bool,
  /// Faint.
  pub faint: bool,
  /// Inverse video.
  pub inverse: bool,
  /// Any underline style.
  pub underline: bool,
  /// Strikethrough.
  pub strikethrough: bool,
  /// Inside the current selection.
  pub selected: bool,
  /// Wide character (width 2).
  pub wide: bool,
}

/// Cursor shape in a snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
  /// Block cursor.
  Block,
  /// Underline cursor.
  Underline,
  /// Bar cursor.
  Bar,
}

/// Cursor position and style in a snapshot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSnapshot {
  /// Column in the viewport, 0-based.
  pub x: u16,
  /// Row in the viewport, 0-based.
  pub y: u16,
  /// Whether the cursor is visible.
  pub visible: bool,
  /// Whether the cursor is blinking.
  pub blinking: bool,
  /// Visual shape.
  pub shape: CursorShape,
}

/// One frame of a pane, row-major `cells` of `rows * cols`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSnapshot {
  /// Monotonic snapshot sequence; the first published frame is 1.
  pub seq: u64,
  /// Viewport columns.
  pub cols: u16,
  /// Viewport rows.
  pub rows: u16,
  /// Cells in row-major order.
  pub cells: Vec<SnapshotCell>,
  /// Cursor when it is in the viewport.
  pub cursor: Option<CursorSnapshot>,
  /// Default background.
  pub background: Rgb,
  /// Default foreground.
  pub foreground: Rgb,
  /// Cursor color when the terminal set one.
  pub cursor_color: Option<Rgb>,
  /// Rows scrolled back from the bottom.
  pub viewport_offset: usize,
  /// Scrollback rows (total minus viewport).
  pub scrollback_rows: usize,
}

impl PaneSnapshot {
  /// Cell at (`x`, `y`), if it is inside the grid.
  pub fn cell(&self, x: u16, y: u16) -> Option<&SnapshotCell> {
    if x >= self.cols || y >= self.rows {
      return None;
    }
    self.cells.get(usize::from(y) * usize::from(self.cols) + usize::from(x))
  }

  /// Visible text of row `y`, trailing spaces trimmed.
  pub fn row_text(&self, y: u16) -> String {
    if y >= self.rows {
      return String::new();
    }
    let start = usize::from(y) * usize::from(self.cols);
    let end = start + usize::from(self.cols);
    let mut text = String::new();
    for cell in self.cells.get(start..end).into_iter().flatten() {
      text.push_str(&cell.text);
    }
    text.trim_end().to_string()
  }

  /// Inclusive cell range in reading order; rows joined by newlines, trailing spaces trimmed per row.
  pub fn selection_text(&self, start: (u16, u16), end: (u16, u16)) -> String {
    let (start, end) = order_cells(start, end);
    if self.cols == 0 || self.rows == 0 {
      return String::new();
    }
    let mut lines = Vec::new();
    for y in start.1..=end.1 {
      if y >= self.rows {
        break;
      }
      let x0 = if y == start.1 { start.0 } else { 0 };
      let x1 = if y == end.1 {
        end.0.min(self.cols.saturating_sub(1))
      } else {
        self.cols.saturating_sub(1)
      };
      let mut line = String::new();
      for x in x0..=x1 {
        if let Some(cell) = self.cell(x, y) {
          line.push_str(&cell.text);
        }
      }
      lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
  }

  /// Inclusive word range at (`x`, `y`) using whitespace boundaries.
  pub fn word_at(&self, x: u16, y: u16) -> Option<((u16, u16), (u16, u16))> {
    let cell = self.cell(x, y)?;
    if is_word_break(&cell.text) {
      return None;
    }
    let mut start_x = x;
    while start_x > 0 {
      let prev = self.cell(start_x - 1, y)?;
      if is_word_break(&prev.text) {
        break;
      }
      start_x -= 1;
    }
    let mut end_x = x;
    while end_x + 1 < self.cols {
      let next = self.cell(end_x + 1, y)?;
      if is_word_break(&next.text) {
        break;
      }
      end_x += 1;
    }
    Some(((start_x, y), (end_x, y)))
  }
}

fn order_cells(start: (u16, u16), end: (u16, u16)) -> ((u16, u16), (u16, u16)) {
  if reading_pos(start) <= reading_pos(end) {
    (start, end)
  } else {
    (end, start)
  }
}

fn reading_pos(cell: (u16, u16)) -> u32 {
  (u32::from(cell.1) << 16) | u32::from(cell.0)
}

fn is_word_break(text: &str) -> bool {
  text.is_empty() || text.chars().all(char::is_whitespace)
}

#[cfg(test)]
mod tests {
  use super::{PaneSnapshot, Rgb, SnapshotCell};

  fn text_cell(ch: char) -> SnapshotCell {
    SnapshotCell {
      text: ch.to_string(),
      ..SnapshotCell::default()
    }
  }

  fn snapshot_from_rows(cols: u16, rows: &[&str]) -> PaneSnapshot {
    let mut cells = Vec::with_capacity(usize::from(cols) * rows.len());
    for row in rows {
      let mut chars: Vec<char> = row.chars().collect();
      chars.resize(usize::from(cols), ' ');
      for ch in chars {
        cells.push(text_cell(ch));
      }
    }
    PaneSnapshot {
      seq: 1,
      cols,
      rows: rows.len() as u16,
      cells,
      cursor: None,
      background: Rgb::default(),
      foreground: Rgb::default(),
      cursor_color: None,
      viewport_offset: 0,
      scrollback_rows: 0,
    }
  }

  #[test]
  fn selection_text_joins_rows_and_trims() {
    let snap = snapshot_from_rows(8, &["hello   ", "world   "]);
    assert_eq!(snap.selection_text((0, 0), (4, 0)), "hello");
    assert_eq!(snap.selection_text((0, 0), (7, 0)), "hello");
    assert_eq!(snap.selection_text((0, 0), (4, 1)), "hello\nworld");
    assert_eq!(snap.selection_text((1, 0), (2, 1)), "ello\nwor");
    assert_eq!(snap.selection_text((4, 1), (0, 0)), "hello\nworld");
  }

  #[test]
  fn word_at_finds_boundaries() {
    let snap = snapshot_from_rows(11, &["hello world"]);
    assert_eq!(snap.word_at(1, 0), Some(((0, 0), (4, 0))));
    assert_eq!(snap.word_at(8, 0), Some(((6, 0), (10, 0))));
    assert_eq!(snap.word_at(5, 0), None);
    assert_eq!(snap.word_at(20, 0), None);
  }
}
