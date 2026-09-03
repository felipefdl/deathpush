use libghostty_vt::render::{CellIterator, RowIterator};
use libghostty_vt::{RenderState, Terminal};

use crate::error::{Error, Result};

/// A VT screen: bytes in, styled rows out. Not `Send`; own it on one thread.
pub struct VtScreen {
  terminal: Terminal<'static, 'static>,
  render: RenderState<'static>,
}

impl VtScreen {
  pub fn new(cols: u16, rows: u16) -> Result<Self> {
    let terminal = Terminal::new(cols, rows).map_err(|err| Error::Other(err.to_string()))?;
    let render = RenderState::new().map_err(|err| Error::Other(err.to_string()))?;
    Ok(Self { terminal, render })
  }

  pub fn write(&mut self, bytes: &[u8]) {
    self.terminal.vt_write(bytes);
  }

  /// Visible rows as plain text, trailing spaces trimmed.
  pub fn text_rows(&mut self) -> Result<Vec<String>> {
    let snapshot = self
      .render
      .update(&self.terminal)
      .map_err(|err| Error::Other(err.to_string()))?;
    let mut rows = RowIterator::new().map_err(|err| Error::Other(err.to_string()))?;
    let mut cells = CellIterator::new().map_err(|err| Error::Other(err.to_string()))?;
    let mut row_iter = rows.update(&snapshot).map_err(|err| Error::Other(err.to_string()))?;
    let mut out = Vec::new();
    while let Some(row) = row_iter.next() {
      let mut text = String::new();
      let mut cell_iter = cells.update(row).map_err(|err| Error::Other(err.to_string()))?;
      while let Some(cell) = cell_iter.next() {
        let graphemes = cell.graphemes().map_err(|err| Error::Other(err.to_string()))?;
        if graphemes.is_empty() {
          text.push(' ');
        } else {
          text.extend(graphemes);
        }
      }
      out.push(text.trim_end().to_string());
    }
    Ok(out)
  }
}

#[cfg(test)]
mod tests {
  use super::VtScreen;

  #[test]
  fn plain_lines_land_on_rows() {
    let mut screen = VtScreen::new(20, 4).unwrap();
    screen.write(b"hello\r\nworld\r\n");
    let rows = screen.text_rows().unwrap();
    assert_eq!(rows[0], "hello");
    assert_eq!(rows[1], "world");
    assert_eq!(rows[2], "");
  }

  #[test]
  fn sgr_sequences_do_not_leak_into_text() {
    let mut screen = VtScreen::new(20, 2).unwrap();
    screen.write(b"\x1b[1;32mgreen\x1b[0m text");
    let rows = screen.text_rows().unwrap();
    assert_eq!(rows[0], "green text");
  }

  #[test]
  fn long_line_wraps_at_columns() {
    let mut screen = VtScreen::new(5, 3).unwrap();
    screen.write(b"abcdefgh");
    let rows = screen.text_rows().unwrap();
    assert_eq!(rows[0], "abcde");
    assert_eq!(rows[1], "fgh");
  }
}
