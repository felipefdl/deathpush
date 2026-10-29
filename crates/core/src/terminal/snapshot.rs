//! Pure terminal snapshot types. Send + Sync; no libghostty types.

use std::sync::Arc;

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

/// One viewport row of a snapshot: `cols` cells. Rows libghostty did not report dirty are shared
/// by `Arc` with the previous snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SnapshotRow {
  /// Cells left to right.
  pub cells: Vec<SnapshotCell>,
}

/// Mouse pointer shape a program asked for with OSC 22.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PointerShape {
  /// Platform default arrow.
  Default,
  /// Context menu.
  ContextMenu,
  /// Help.
  Help,
  /// Pointing hand (link).
  Pointer,
  /// Working in the background.
  Progress,
  /// Busy.
  Wait,
  /// Cell selection plus.
  Cell,
  /// Crosshair.
  Crosshair,
  /// Text caret; the initial shape.
  #[default]
  Text,
  /// Vertical text caret.
  VerticalText,
  /// Alias or shortcut.
  Alias,
  /// Copy.
  Copy,
  /// Move.
  Move,
  /// Drop not possible here.
  NoDrop,
  /// Action not allowed.
  NotAllowed,
  /// Open hand.
  Grab,
  /// Closed hand.
  Grabbing,
  /// Scroll in any direction.
  AllScroll,
  /// Resize columns.
  ColResize,
  /// Resize rows.
  RowResize,
  /// Resize north edge.
  NResize,
  /// Resize east edge.
  EResize,
  /// Resize south edge.
  SResize,
  /// Resize west edge.
  WResize,
  /// Resize north-east corner.
  NeResize,
  /// Resize north-west corner.
  NwResize,
  /// Resize south-east corner.
  SeResize,
  /// Resize south-west corner.
  SwResize,
  /// Resize east-west.
  EwResize,
  /// Resize north-south.
  NsResize,
  /// Resize along the north-east diagonal.
  NeswResize,
  /// Resize along the north-west diagonal.
  NwseResize,
  /// Zoom in.
  ZoomIn,
  /// Zoom out.
  ZoomOut,
}

/// Decoded pixels of one Kitty graphics image frame, 8-bit RGBA, row by row from the top-left with
/// no padding. Shared by `Arc` between snapshots while `id` and `generation` stay the same.
///
/// Equality compares `id` and `generation` only: libghostty never reuses a generation, so equal
/// keys mean equal pixels and a snapshot comparison never walks the bytes.
#[derive(Debug)]
pub struct ImagePixels {
  /// Image id the program chose or the terminal assigned.
  pub id: u32,
  /// libghostty's generation of the image; changes whenever its pixels do, animation frames included.
  pub generation: u64,
  /// Width in pixels.
  pub width: u32,
  /// Height in pixels.
  pub height: u32,
  /// `width * height * 4` bytes, RGBA.
  pub rgba: Vec<u8>,
}

impl PartialEq for ImagePixels {
  fn eq(&self, other: &Self) -> bool {
    self.id == other.id && self.generation == other.generation
  }
}

impl Eq for ImagePixels {}

/// Where an image is drawn relative to the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ImageLayer {
  /// Below the cell background colors.
  BelowBackground,
  /// Above the cell backgrounds, below the text.
  BelowText,
  /// Above the text.
  AboveText,
}

/// One placement of a Kitty graphics image in the viewport, in the terminal's own pixels (the
/// cell size the pane last gave libghostty, [`PaneSnapshot::cell_width`] by
/// [`PaneSnapshot::cell_height`]).
///
/// The rectangle is reported at its full size: it can start left of or above the viewport and run
/// past its right and bottom edges, so the painter clips it to the pane. A placement made of
/// Unicode placeholder cells is one placement per row of cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotImage {
  /// The decoded image.
  pub pixels: Arc<ImagePixels>,
  /// Draw layer.
  pub layer: ImageLayer,
  /// Order within the layer, lowest first.
  pub z: i32,
  /// Column of the top-left cell, 0 is the leftmost column; negative when it started off the left.
  pub col: i32,
  /// Row of the top-left cell, 0 is the top viewport row; negative when it started above it.
  pub row: i32,
  /// Columns of cells covered.
  pub grid_cols: u32,
  /// Rows of cells covered.
  pub grid_rows: u32,
  /// Pixels from the left edge of the top-left cell to the image.
  pub offset_x: u32,
  /// Pixels from the top edge of the top-left cell to the image.
  pub offset_y: u32,
  /// Width to draw at, in pixels.
  pub width: u32,
  /// Height to draw at, in pixels.
  pub height: u32,
  /// Left edge of the part of the image to draw, in image pixels.
  pub source_x: u32,
  /// Top edge of the part of the image to draw, in image pixels.
  pub source_y: u32,
  /// Width of the part of the image to draw, in image pixels.
  pub source_width: u32,
  /// Height of the part of the image to draw, in image pixels.
  pub source_height: u32,
}

/// One run of a search match inside a single viewport row, columns inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchSpan {
  /// Viewport row.
  pub y: u16,
  /// First column.
  pub x0: u16,
  /// Last column.
  pub x1: u16,
}

/// Terminal search results as of one snapshot.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SearchSnapshot {
  /// Matches in scrollback and screen.
  pub total: usize,
  /// Position of the selected match counted from the top of the scrollback, starting at 1.
  pub current: Option<usize>,
  /// Viewport runs of every match visible on screen.
  pub matches: Vec<SearchSpan>,
  /// Viewport runs of the selected match.
  pub current_match: Vec<SearchSpan>,
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

/// One frame of a pane: `rows` lines of `cols` cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneSnapshot {
  /// Monotonic snapshot sequence; the first published frame is 1.
  pub seq: u64,
  /// Viewport columns.
  pub cols: u16,
  /// Viewport rows.
  pub rows: u16,
  /// Viewport lines top to bottom, `rows` of them.
  pub lines: Vec<Arc<SnapshotRow>>,
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
  /// True when a BEL (`0x07`) arrived since the previous published snapshot.
  pub bell: bool,
  /// Pointer shape from OSC 22.
  pub pointer: PointerShape,
  /// Search results while a search is open with a non-empty query.
  pub search: Option<SearchSnapshot>,
  /// Cell width in the pixels `images` are measured in.
  pub cell_width: u32,
  /// Cell height in the pixels `images` are measured in.
  pub cell_height: u32,
  /// Kitty graphics placements on screen. The same `Arc` as the previous snapshot while the
  /// terminal reported no change to them, so clean frames share the list and every pixel buffer.
  pub images: Arc<Vec<SnapshotImage>>,
}

impl PaneSnapshot {
  /// Cell at (`x`, `y`), if it is inside the grid.
  pub fn cell(&self, x: u16, y: u16) -> Option<&SnapshotCell> {
    if x >= self.cols || y >= self.rows {
      return None;
    }
    self.lines.get(usize::from(y))?.cells.get(usize::from(x))
  }

  /// Visible text of row `y`, trailing spaces trimmed.
  pub fn row_text(&self, y: u16) -> String {
    let Some(line) = self.lines.get(usize::from(y)).filter(|_| y < self.rows) else {
      return String::new();
    };
    let mut text = String::new();
    for cell in &line.cells {
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
  use std::sync::Arc;

  use super::{PaneSnapshot, PointerShape, Rgb, SnapshotCell, SnapshotRow};

  fn text_cell(ch: char) -> SnapshotCell {
    SnapshotCell {
      text: ch.to_string(),
      ..SnapshotCell::default()
    }
  }

  fn snapshot_from_rows(cols: u16, rows: &[&str]) -> PaneSnapshot {
    let lines = rows
      .iter()
      .map(|row| {
        let mut chars: Vec<char> = row.chars().collect();
        chars.resize(usize::from(cols), ' ');
        Arc::new(SnapshotRow {
          cells: chars.into_iter().map(text_cell).collect(),
        })
      })
      .collect();
    PaneSnapshot {
      seq: 1,
      cols,
      rows: rows.len() as u16,
      lines,
      cursor: None,
      background: Rgb::default(),
      foreground: Rgb::default(),
      cursor_color: None,
      viewport_offset: 0,
      scrollback_rows: 0,
      bell: false,
      pointer: PointerShape::default(),
      search: None,
      cell_width: 8,
      cell_height: 16,
      images: Arc::default(),
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
