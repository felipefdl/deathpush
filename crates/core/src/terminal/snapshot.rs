//! Pure terminal snapshot types. Send + Sync; no libghostty types.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rgb(pub u8, pub u8, pub u8);

#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnapshotCell {
  pub text: String,
  pub fg: Option<Rgb>,
  pub bg: Option<Rgb>,
  pub bold: bool,
  pub italic: bool,
  pub faint: bool,
  pub inverse: bool,
  pub underline: bool,
  pub strikethrough: bool,
  pub selected: bool,
  pub wide: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorShape {
  Block,
  Underline,
  Bar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorSnapshot {
  pub x: u16,
  pub y: u16,
  pub visible: bool,
  pub blinking: bool,
  pub shape: CursorShape,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSnapshot {
  pub seq: u64,
  pub cols: u16,
  pub rows: u16,
  pub cells: Vec<SnapshotCell>,
  pub cursor: Option<CursorSnapshot>,
  pub background: Rgb,
  pub foreground: Rgb,
  pub cursor_color: Option<Rgb>,
  pub viewport_offset: usize,
  pub scrollback_rows: usize,
}

impl PaneSnapshot {
  pub fn cell(&self, x: u16, y: u16) -> Option<&SnapshotCell> {
    if x >= self.cols || y >= self.rows {
      return None;
    }
    self.cells.get(usize::from(y) * usize::from(self.cols) + usize::from(x))
  }

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
