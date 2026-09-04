//! One OS thread per terminal pane. libghostty types stay here; the app sees snapshots.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};

use libghostty_vt::key::{self, Encoder as KeyEncoder, Event as KeyEvent};
use libghostty_vt::mouse::{self, Encoder as MouseEncoder, Event as MouseEvent};
use libghostty_vt::render::{CellIterator, CursorVisualStyle, RowIterator};
use libghostty_vt::screen::CellWide;
use libghostty_vt::style::{PaletteIndex, RgbColor, Underline};
use libghostty_vt::terminal::{Mode, ScrollViewport};
use libghostty_vt::{RenderState, Terminal};

use crate::error::{Error, Result};
use crate::terminal::snapshot::{CursorShape, CursorSnapshot, PaneSnapshot, Rgb, SnapshotCell};

/// Keyboard modifiers on a pane key or mouse event.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct KeyMods {
  /// Shift is held.
  pub shift: bool,
  /// Alt/Option is held.
  pub alt: bool,
  /// Control is held.
  pub ctrl: bool,
  /// Super/Command/Windows is held.
  pub super_: bool,
}

/// A key event from the focused pane, using gpui key names.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyInput {
  /// gpui key name, such as `"a"`, `"enter"`, or `"f1"`.
  pub key: String,
  /// Layout character from `key_char`, when present.
  pub text: Option<String>,
  /// Modifiers held with the key.
  pub mods: KeyMods,
  /// `true` for press, `false` for release.
  pub press: bool,
}

/// Mouse action forwarded to the pane thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
  /// Button down.
  Press,
  /// Button up.
  Release,
  /// Pointer moved.
  Motion,
}

/// Mouse button forwarded to the pane thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
  /// Primary button.
  Left,
  /// Middle button.
  Middle,
  /// Secondary button.
  Right,
  /// Wheel up, encoded as button four when tracking.
  WheelUp,
  /// Wheel down, encoded as button five when tracking.
  WheelDown,
}

/// A mouse event in cell coordinates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MouseInput {
  /// Press, release, or motion.
  pub action: MouseAction,
  /// Button for the event, if any.
  pub button: Option<MouseButton>,
  /// Column, 0-based.
  pub x: u16,
  /// Row, 0-based.
  pub y: u16,
  /// Modifiers held with the mouse event.
  pub mods: KeyMods,
}

/// Work for the pane thread.
#[derive(Debug)]
pub enum PaneCommand {
  /// Bytes from the PTY, written into the VT parser.
  Bytes(Vec<u8>),
  /// Clipboard text written to the PTY, bracketed when the terminal has paste bracketing on.
  Paste(String),
  /// Committed UTF-8 from IME or character input, written to the PTY without paste bracketing.
  Text(String),
  /// Encode a key and write the result to the PTY.
  Key(KeyInput),
  /// Encode a mouse event when tracking is on.
  Mouse(MouseInput),
  /// Resize the terminal grid and cell pixel size.
  Resize {
    /// Column count.
    cols: u16,
    /// Row count.
    rows: u16,
    /// Cell width in pixels.
    cell_w: u32,
    /// Cell height in pixels.
    cell_h: u32,
  },
  /// Scroll the viewport by a row delta (negative is up).
  Scroll(isize),
  /// Pin the viewport to the active area.
  ScrollToBottom,
  /// Set the scrollback byte limit; `None` is unlimited.
  SetScrollbackBytes(Option<usize>),
  /// Apply theme colors to libghostty defaults and publish a fresh snapshot.
  SetColors {
    /// Default foreground.
    foreground: Rgb,
    /// Default background.
    background: Rgb,
    /// Default cursor color.
    cursor: Rgb,
    /// Palette entries 0..15.
    ansi: [Rgb; 16],
  },
  /// Stop the pane thread.
  Shutdown,
}

enum CommandEffect {
  Shutdown,
  Dirty,
  Clean,
}

enum PaneMsg {
  Bytes(Vec<u8>),
  Command(PaneCommand),
}

const BATCH_MESSAGE_LIMIT: usize = 256;
const BATCH_BYTE_LIMIT: usize = 64 * 1024;

struct PaneIo {
  slot: Arc<Mutex<Option<Arc<PaneSnapshot>>>>,
  mouse_tracking: Arc<AtomicBool>,
  shutdown: Arc<AtomicBool>,
}

/// Bytes the pane thread wants written to the PTY (encoded keys, mouse reports, pasted text).
pub type PtyWriter = Box<dyn Fn(Vec<u8>) + Send + 'static>;

/// Handle to a pane thread. Dropping it shuts the thread down; the join runs in the background.
pub struct PaneHandle {
  tx: Sender<PaneMsg>,
  slot: Arc<Mutex<Option<Arc<PaneSnapshot>>>>,
  mouse_tracking: Arc<AtomicBool>,
  shutdown: Arc<AtomicBool>,
  join: Option<JoinHandle<()>>,
}

impl PaneHandle {
  /// Spawns the pane thread.
  ///
  /// `writer` receives encoded PTY input and libghostty `on_pty_write` responses on the pane thread.
  /// `wake` runs on the pane thread after every new snapshot so the app can schedule a redraw.
  /// It must not block and must hold only weak or lifetime-safe app state. Shutdown is an atomic
  /// flag plus a [`PaneCommand::Shutdown`] on the same FIFO; [`Drop`] sets the flag, sends, and
  /// joins on a background thread so the UI is not blocked. Shared snapshot state stays alive
  /// until that join completes.
  pub fn spawn(
    cols: u16,
    rows: u16,
    scrollback_bytes: Option<usize>,
    writer: PtyWriter,
    wake: Box<dyn Fn() + Send + 'static>,
  ) -> Result<PaneHandle> {
    let (tx, rx) = mpsc::channel();
    let slot = Arc::new(Mutex::new(None));
    let mouse_tracking = Arc::new(AtomicBool::new(false));
    let shutdown = Arc::new(AtomicBool::new(false));
    let (ready_tx, ready_rx) = mpsc::channel();
    let io = PaneIo {
      slot: Arc::clone(&slot),
      mouse_tracking: Arc::clone(&mouse_tracking),
      shutdown: Arc::clone(&shutdown),
    };
    let join = thread::Builder::new()
      .name("dp-vt-pane".into())
      .spawn(
        move || match PaneThread::new(cols, rows, scrollback_bytes, writer, wake, io) {
          Ok(pane) => {
            let _ = ready_tx.send(Ok(()));
            pane.run(rx);
          }
          Err(err) => {
            let _ = ready_tx.send(Err(err));
          }
        },
      )
      .map_err(|err| Error::Other(err.to_string()))?;
    match ready_rx.recv() {
      Ok(Ok(())) => Ok(PaneHandle {
        tx,
        slot,
        mouse_tracking,
        shutdown,
        join: Some(join),
      }),
      Ok(Err(err)) => {
        let _ = join.join();
        Err(err)
      }
      Err(_) => {
        let _ = join.join();
        Err(Error::Other("pane thread exited before ready".into()))
      }
    }
  }

  /// Queue a command for the pane thread. Never blocks.
  ///
  /// Shares FIFO order with [`Self::push_bytes`] on one unbounded channel.
  pub fn send(&self, command: PaneCommand) {
    if matches!(command, PaneCommand::Shutdown) {
      self.shutdown.store(true, Ordering::Release);
    }
    let msg = match command {
      PaneCommand::Bytes(bytes) => PaneMsg::Bytes(bytes),
      command => PaneMsg::Command(command),
    };
    let _ = self.tx.send(msg);
  }

  /// Append PTY output for the pane thread. Never blocks.
  ///
  /// Shares FIFO order with [`Self::send`] on the same unbounded channel.
  pub fn push_bytes(&self, bytes: &[u8]) {
    if bytes.is_empty() {
      return;
    }
    let _ = self.tx.send(PaneMsg::Bytes(bytes.to_vec()));
  }

  /// Latest published snapshot, if any.
  pub fn snapshot(&self) -> Option<Arc<PaneSnapshot>> {
    self.slot.lock().ok()?.clone()
  }

  /// Whether the terminal currently reports mouse tracking (the element then forwards mouse events instead of selecting).
  pub fn mouse_tracking(&self) -> bool {
    self.mouse_tracking.load(Ordering::Acquire)
  }
}

/// Sets the shutdown flag and sends [`PaneCommand::Shutdown`]. The pane thread is joined by a
/// process-wide reaper so Drop does not block the UI. The pane thread owns the snapshot Arcs
/// until it exits.
impl Drop for PaneHandle {
  fn drop(&mut self) {
    self.shutdown.store(true, Ordering::Release);
    let _ = self.tx.send(PaneMsg::Command(PaneCommand::Shutdown));
    if let Some(join) = self.join.take() {
      enqueue_pane_join(join);
    }
  }
}

fn enqueue_pane_join(join: JoinHandle<()>) {
  static TX: OnceLock<Sender<JoinHandle<()>>> = OnceLock::new();
  let tx = TX.get_or_init(|| {
    let (tx, rx) = mpsc::channel::<JoinHandle<()>>();
    let _ = thread::Builder::new().name("dp-vt-pane-join".into()).spawn(move || {
      while let Ok(handle) = rx.recv() {
        let _ = handle.join();
      }
    });
    tx
  });
  if let Err(err) = tx.send(join) {
    drop(err.0);
  }
}

struct PaneThread {
  terminal: Terminal<'static, 'static>,
  render: RenderState<'static>,
  row_iter: RowIterator<'static>,
  cell_iter: CellIterator<'static>,
  key_encoder: KeyEncoder<'static>,
  key_event: KeyEvent<'static>,
  mouse_encoder: MouseEncoder<'static>,
  mouse_event: MouseEvent<'static>,
  writer: Rc<PtyWriter>,
  wake: Box<dyn Fn() + Send + 'static>,
  slot: Arc<Mutex<Option<Arc<PaneSnapshot>>>>,
  mouse_tracking: Arc<AtomicBool>,
  seq: u64,
  cols: u16,
  rows: u16,
  cell_w: u32,
  cell_h: u32,
  pty_buf: Vec<u8>,
  last_snapshot_error: Option<String>,
  shutdown: Arc<AtomicBool>,
  bell: Rc<Cell<bool>>,
}

impl PaneThread {
  fn new(
    cols: u16,
    rows: u16,
    scrollback_bytes: Option<usize>,
    writer: PtyWriter,
    wake: Box<dyn Fn() + Send + 'static>,
    io: PaneIo,
  ) -> Result<Self> {
    let (cols, rows) = clamp_grid(cols, rows);
    let writer = Rc::new(writer);
    let bell = Rc::new(Cell::new(false));
    let mut terminal = Terminal::new(cols, rows).map_err(vt_err)?;
    {
      let writer = Rc::clone(&writer);
      terminal
        .on_pty_write(move |_term, data| {
          if !data.is_empty() {
            (*writer)(data.to_vec());
          }
        })
        .map_err(vt_err)?;
    }
    {
      let bell = Rc::clone(&bell);
      terminal
        .on_bell(move |_term| {
          bell.set(true);
        })
        .map_err(vt_err)?;
    }
    let cell_w = 8;
    let cell_h = 16;
    if let Err(err) = terminal.resize(cols, rows, cell_w, cell_h) {
      tracing::warn!(%err, "terminal pixel resize failed");
    }
    if let Err(err) = terminal.set_scrollback_max_bytes(scrollback_bytes) {
      tracing::warn!(%err, "set scrollback failed");
    }
    Ok(Self {
      terminal,
      render: RenderState::new().map_err(vt_err)?,
      row_iter: RowIterator::new().map_err(vt_err)?,
      cell_iter: CellIterator::new().map_err(vt_err)?,
      key_encoder: KeyEncoder::new().map_err(vt_err)?,
      key_event: KeyEvent::new().map_err(vt_err)?,
      mouse_encoder: MouseEncoder::new().map_err(vt_err)?,
      mouse_event: MouseEvent::new().map_err(vt_err)?,
      writer,
      wake,
      slot: io.slot,
      mouse_tracking: io.mouse_tracking,
      seq: 0,
      cols,
      rows,
      cell_w,
      cell_h,
      pty_buf: Vec::new(),
      last_snapshot_error: None,
      shutdown: io.shutdown,
      bell,
    })
  }

  fn shutting_down(&self) -> bool {
    self.shutdown.load(Ordering::Acquire)
  }

  fn apply_msg(&mut self, msg: PaneMsg, byte_count: &mut usize, dirty: &mut bool) -> bool {
    match msg {
      PaneMsg::Bytes(data) => {
        *byte_count = byte_count.saturating_add(data.len());
        self.terminal.vt_write(&data);
        *dirty = true;
        false
      }
      PaneMsg::Command(command) => match self.apply(command) {
        CommandEffect::Shutdown => true,
        CommandEffect::Dirty => {
          *dirty = true;
          false
        }
        CommandEffect::Clean => false,
      },
    }
  }

  fn run(mut self, rx: Receiver<PaneMsg>) {
    loop {
      if self.shutting_down() {
        break;
      }
      let Ok(first) = rx.recv() else {
        break;
      };
      if self.shutting_down() {
        break;
      }
      let mut messages = 1usize;
      let mut byte_count = 0usize;
      let mut dirty = false;
      if self.apply_msg(first, &mut byte_count, &mut dirty) {
        break;
      }
      while messages < BATCH_MESSAGE_LIMIT && byte_count < BATCH_BYTE_LIMIT {
        if self.shutting_down() {
          return;
        }
        match rx.try_recv() {
          Ok(msg) => {
            messages += 1;
            if self.apply_msg(msg, &mut byte_count, &mut dirty) {
              return;
            }
          }
          Err(TryRecvError::Empty) => break,
          Err(TryRecvError::Disconnected) => return,
        }
      }
      if self.shutting_down() {
        break;
      }
      if dirty {
        self.update_mouse_tracking();
        self.publish();
      }
    }
  }

  fn apply(&mut self, command: PaneCommand) -> CommandEffect {
    match command {
      PaneCommand::Shutdown => CommandEffect::Shutdown,
      PaneCommand::Bytes(bytes) => {
        self.terminal.vt_write(&bytes);
        CommandEffect::Dirty
      }
      PaneCommand::Paste(text) => {
        self.handle_paste(text);
        CommandEffect::Clean
      }
      PaneCommand::Text(text) => {
        self.handle_text(text);
        CommandEffect::Clean
      }
      PaneCommand::Key(input) => {
        self.handle_key(input);
        CommandEffect::Clean
      }
      PaneCommand::Mouse(input) => {
        self.handle_mouse(input);
        CommandEffect::Clean
      }
      PaneCommand::Resize {
        cols,
        rows,
        cell_w,
        cell_h,
      } => {
        let (cols, rows) = clamp_grid(cols, rows);
        self.cols = cols;
        self.rows = rows;
        self.cell_w = cell_w.max(1);
        self.cell_h = cell_h.max(1);
        if let Err(err) = self.terminal.resize(cols, rows, self.cell_w, self.cell_h) {
          tracing::warn!(%err, "terminal resize failed");
        }
        CommandEffect::Dirty
      }
      PaneCommand::Scroll(delta) => {
        self.terminal.scroll_viewport(ScrollViewport::Delta(delta));
        CommandEffect::Dirty
      }
      PaneCommand::ScrollToBottom => {
        self.terminal.scroll_viewport(ScrollViewport::Bottom);
        CommandEffect::Dirty
      }
      PaneCommand::SetScrollbackBytes(bytes) => {
        if let Err(err) = self.terminal.set_scrollback_max_bytes(bytes) {
          tracing::warn!(%err, "set scrollback failed");
        }
        CommandEffect::Dirty
      }
      PaneCommand::SetColors {
        foreground,
        background,
        cursor,
        ansi,
      } => {
        self.apply_theme_colors(foreground, background, cursor, ansi);
        CommandEffect::Dirty
      }
    }
  }

  fn apply_theme_colors(&mut self, foreground: Rgb, background: Rgb, cursor: Rgb, ansi: [Rgb; 16]) {
    let to_rgb = |Rgb(r, g, b)| RgbColor { r, g, b };
    if let Err(err) = self
      .terminal
      .set_default_fg_color(Some(to_rgb(foreground)))
      .and_then(|term| term.set_default_bg_color(Some(to_rgb(background))))
      .and_then(|term| term.set_default_cursor_color(Some(to_rgb(cursor))))
    {
      tracing::warn!(%err, "set terminal default colors failed");
    }
    match self.terminal.default_color_palette() {
      Ok(mut palette) => {
        for (index, color) in ansi.into_iter().enumerate() {
          palette.set(PaletteIndex(index as u8), to_rgb(color));
        }
        if let Err(err) = self.terminal.set_default_color_palette(Some(palette)) {
          tracing::warn!(%err, "set terminal palette failed");
        }
      }
      Err(err) => tracing::warn!(%err, "read terminal palette failed"),
    }
  }

  fn handle_text(&mut self, text: String) {
    if text.is_empty() {
      return;
    }
    self.pty_buf.clear();
    self.pty_buf.extend_from_slice(text.as_bytes());
    self.flush_pty();
  }

  fn handle_paste(&mut self, text: String) {
    let bracketed = self.terminal.mode(Mode::BRACKETED_PASTE).unwrap_or(false);
    self.pty_buf.clear();
    if bracketed {
      self.pty_buf.extend_from_slice(b"\x1b[200~");
      self.pty_buf.extend_from_slice(text.as_bytes());
      self.pty_buf.extend_from_slice(b"\x1b[201~");
    } else {
      self.pty_buf.extend_from_slice(text.as_bytes());
    }
    self.flush_pty();
  }

  fn handle_key(&mut self, input: KeyInput) {
    let Some(key) = key_from_name(&input.key) else {
      return;
    };
    let action = if input.press {
      key::Action::Press
    } else {
      key::Action::Release
    };
    let text = if input.press { printable_utf8(input.text) } else { None };
    self
      .key_event
      .set_action(action)
      .set_key(key)
      .set_mods(mods_from(&input.mods))
      .set_utf8(text);
    self.pty_buf.clear();
    if let Err(err) = self
      .key_encoder
      .set_options_from_terminal(&self.terminal)
      .encode_to_vec(&self.key_event, &mut self.pty_buf)
    {
      tracing::warn!(%err, "key encode failed");
      return;
    }
    self.flush_pty();
  }

  fn handle_mouse(&mut self, input: MouseInput) {
    if !self.mouse_tracking_now() {
      return;
    }
    let action = match input.action {
      MouseAction::Press => mouse::Action::Press,
      MouseAction::Release => mouse::Action::Release,
      MouseAction::Motion => mouse::Action::Motion,
    };
    let cell_w = self.cell_w.max(1) as f32;
    let cell_h = self.cell_h.max(1) as f32;
    self
      .mouse_event
      .set_action(action)
      .set_button(input.button.map(map_mouse_button))
      .set_mods(mods_from(&input.mods))
      .set_position(mouse::Position {
        x: (f32::from(input.x) + 0.5) * cell_w,
        y: (f32::from(input.y) + 0.5) * cell_h,
      });
    let any_pressed = matches!(input.action, MouseAction::Press)
      || (matches!(input.action, MouseAction::Motion) && input.button.is_some());
    self.pty_buf.clear();
    if let Err(err) = self
      .mouse_encoder
      .set_options_from_terminal(&self.terminal)
      .set_size(mouse::EncoderSize {
        screen_width: u32::from(self.cols) * self.cell_w.max(1),
        screen_height: u32::from(self.rows) * self.cell_h.max(1),
        cell_width: self.cell_w.max(1),
        cell_height: self.cell_h.max(1),
        padding_top: 0,
        padding_bottom: 0,
        padding_right: 0,
        padding_left: 0,
      })
      .set_any_button_pressed(any_pressed)
      .encode_to_vec(&self.mouse_event, &mut self.pty_buf)
    {
      tracing::warn!(%err, "mouse encode failed");
      return;
    }
    self.flush_pty();
  }

  fn flush_pty(&mut self) {
    if self.pty_buf.is_empty() {
      return;
    }
    (*self.writer)(std::mem::take(&mut self.pty_buf));
  }

  fn mouse_tracking_now(&self) -> bool {
    match self.terminal.is_mouse_tracking() {
      Ok(tracking) => tracking,
      Err(err) => {
        tracing::warn!(%err, "mouse tracking query failed");
        false
      }
    }
  }

  fn update_mouse_tracking(&self) {
    self.mouse_tracking.store(self.mouse_tracking_now(), Ordering::Release);
  }

  fn publish(&mut self) {
    match self.build_snapshot() {
      Ok(snapshot) => {
        self.last_snapshot_error = None;
        self.bell.set(false);
        let snapshot = Arc::new(snapshot);
        if let Ok(mut slot) = self.slot.lock() {
          *slot = Some(Arc::clone(&snapshot));
        }
        (self.wake)();
      }
      Err(err) => {
        let kind = err.to_string();
        if self.last_snapshot_error.as_deref() != Some(kind.as_str()) {
          tracing::warn!(%err, "terminal snapshot failed");
          self.last_snapshot_error = Some(kind);
        }
      }
    }
  }

  fn build_snapshot(&mut self) -> Result<PaneSnapshot> {
    let (cols, rows, cells, cursor, default_fg, default_bg, cursor_color) = {
      let snap = self.render.update(&self.terminal).map_err(vt_err)?;
      let cols = snap.cols().map_err(vt_err)?;
      let rows = snap.rows().map_err(vt_err)?;
      let colors = snap.colors().map_err(vt_err)?;
      let cursor_visible = snap.cursor_visible().map_err(vt_err)?;
      let cursor_blinking = snap.cursor_blinking().map_err(vt_err)?;
      let cursor_style = snap.cursor_visual_style().map_err(vt_err)?;
      let cursor_viewport = snap.cursor_viewport().map_err(vt_err)?;
      let cursor_color = snap.cursor_color().map_err(vt_err)?;
      let shape = match cursor_style {
        CursorVisualStyle::Bar => CursorShape::Bar,
        CursorVisualStyle::Underline => CursorShape::Underline,
        CursorVisualStyle::Block | CursorVisualStyle::BlockHollow => CursorShape::Block,
        _ => CursorShape::Block,
      };
      let cursor = cursor_viewport.map(|viewport| CursorSnapshot {
        x: viewport.x,
        y: viewport.y,
        visible: cursor_visible,
        blinking: cursor_blinking,
        shape,
      });
      let default_fg = Rgb(colors.foreground.r, colors.foreground.g, colors.foreground.b);
      let default_bg = Rgb(colors.background.r, colors.background.g, colors.background.b);
      let mut cells = Vec::with_capacity(usize::from(cols) * usize::from(rows));
      let mut row_iter = self.row_iter.update(&snap).map_err(vt_err)?;
      while let Some(row) = row_iter.next() {
        let mut cell_iter = self.cell_iter.update(row).map_err(vt_err)?;
        while let Some(cell) = cell_iter.next() {
          let mut text = String::new();
          if cell.graphemes_len().map_err(vt_err)? == 0 {
            text.push(' ');
          } else {
            cell.graphemes_utf8(&mut text).map_err(vt_err)?;
            if text.is_empty() {
              text.push(' ');
            }
          }
          let style = cell.style().map_err(vt_err)?;
          let fg = cell
            .fg_color()
            .map_err(vt_err)?
            .map(|color| Rgb(color.r, color.g, color.b))
            .or(Some(default_fg));
          let bg = cell
            .bg_color()
            .map_err(vt_err)?
            .map(|color| Rgb(color.r, color.g, color.b))
            .or(Some(default_bg));
          let wide = cell.raw_cell().map_err(vt_err)?.wide().map_err(vt_err)? == CellWide::Wide;
          cells.push(SnapshotCell {
            text,
            fg,
            bg,
            bold: style.bold,
            italic: style.italic,
            faint: style.faint,
            inverse: style.inverse,
            underline: style.underline != Underline::None,
            strikethrough: style.strikethrough,
            selected: cell.is_selected().map_err(vt_err)?,
            wide,
          });
        }
      }
      (cols, rows, cells, cursor, default_fg, default_bg, cursor_color)
    };
    let bar = self.terminal.scrollbar().map_err(vt_err)?;
    let viewport_offset = bar.total.saturating_sub(bar.offset.saturating_add(bar.len)) as usize;
    let scrollback_rows = self.terminal.scrollback_rows().map_err(vt_err)?;
    self.seq += 1;
    Ok(PaneSnapshot {
      seq: self.seq,
      cols,
      rows,
      cells,
      cursor,
      background: default_bg,
      foreground: default_fg,
      cursor_color: cursor_color.map(|color| Rgb(color.r, color.g, color.b)),
      viewport_offset,
      scrollback_rows,
      bell: self.bell.get(),
    })
  }
}

fn vt_err(err: impl std::fmt::Display) -> Error {
  Error::Other(err.to_string())
}

fn clamp_grid(cols: u16, rows: u16) -> (u16, u16) {
  (cols.max(1), rows.max(1))
}

fn printable_utf8(text: Option<String>) -> Option<String> {
  let text = text?;
  let filtered: String = text.chars().filter(|ch| is_printable_char(*ch)).collect();
  if filtered.is_empty() { None } else { Some(filtered) }
}

fn is_printable_char(ch: char) -> bool {
  !matches!(ch, '\u{0000}'..='\u{001F}' | '\u{007F}' | '\u{F700}'..='\u{F8FF}')
}

fn mods_from(mods: &KeyMods) -> key::Mods {
  let mut out = key::Mods::empty();
  if mods.shift {
    out |= key::Mods::SHIFT;
  }
  if mods.alt {
    out |= key::Mods::ALT;
  }
  if mods.ctrl {
    out |= key::Mods::CTRL;
  }
  if mods.super_ {
    out |= key::Mods::SUPER;
  }
  out
}

fn map_mouse_button(button: MouseButton) -> mouse::Button {
  match button {
    MouseButton::Left => mouse::Button::Left,
    MouseButton::Middle => mouse::Button::Middle,
    MouseButton::Right => mouse::Button::Right,
    MouseButton::WheelUp => mouse::Button::Four,
    MouseButton::WheelDown => mouse::Button::Five,
  }
}

/// gpui key names to libghostty keys: letters, digits, punctuation, "enter", "escape", "tab", "backspace", "delete", "space", arrows, "home", "end", "pageup", "pagedown", "insert", "f1".."f24"; None for unknown names.
pub fn key_from_name(name: &str) -> Option<libghostty_vt::key::Key> {
  use libghostty_vt::key::Key;
  const LETTERS: [Key; 26] = [
    Key::A,
    Key::B,
    Key::C,
    Key::D,
    Key::E,
    Key::F,
    Key::G,
    Key::H,
    Key::I,
    Key::J,
    Key::K,
    Key::L,
    Key::M,
    Key::N,
    Key::O,
    Key::P,
    Key::Q,
    Key::R,
    Key::S,
    Key::T,
    Key::U,
    Key::V,
    Key::W,
    Key::X,
    Key::Y,
    Key::Z,
  ];
  const DIGITS: [Key; 10] = [
    Key::Digit0,
    Key::Digit1,
    Key::Digit2,
    Key::Digit3,
    Key::Digit4,
    Key::Digit5,
    Key::Digit6,
    Key::Digit7,
    Key::Digit8,
    Key::Digit9,
  ];
  const FKEYS: [Key; 24] = [
    Key::F1,
    Key::F2,
    Key::F3,
    Key::F4,
    Key::F5,
    Key::F6,
    Key::F7,
    Key::F8,
    Key::F9,
    Key::F10,
    Key::F11,
    Key::F12,
    Key::F13,
    Key::F14,
    Key::F15,
    Key::F16,
    Key::F17,
    Key::F18,
    Key::F19,
    Key::F20,
    Key::F21,
    Key::F22,
    Key::F23,
    Key::F24,
  ];
  if name.len() == 1 {
    let byte = name.as_bytes()[0];
    return match byte {
      b'a'..=b'z' => Some(LETTERS[usize::from(byte - b'a')]),
      b'A'..=b'Z' => Some(LETTERS[usize::from(byte - b'A')]),
      b'0'..=b'9' => Some(DIGITS[usize::from(byte - b'0')]),
      b'`' => Some(Key::Backquote),
      b'-' => Some(Key::Minus),
      b'=' => Some(Key::Equal),
      b'[' => Some(Key::BracketLeft),
      b']' => Some(Key::BracketRight),
      b'\\' => Some(Key::Backslash),
      b';' => Some(Key::Semicolon),
      b'\'' => Some(Key::Quote),
      b',' => Some(Key::Comma),
      b'.' => Some(Key::Period),
      b'/' => Some(Key::Slash),
      b' ' => Some(Key::Space),
      b'!' => Some(Key::Digit1),
      b'@' => Some(Key::Digit2),
      b'#' => Some(Key::Digit3),
      b'$' => Some(Key::Digit4),
      b'%' => Some(Key::Digit5),
      b'^' => Some(Key::Digit6),
      b'&' => Some(Key::Digit7),
      b'*' => Some(Key::Digit8),
      b'(' => Some(Key::Digit9),
      b')' => Some(Key::Digit0),
      b'_' => Some(Key::Minus),
      b'+' => Some(Key::Equal),
      b'{' => Some(Key::BracketLeft),
      b'}' => Some(Key::BracketRight),
      b'|' => Some(Key::Backslash),
      b':' => Some(Key::Semicolon),
      b'"' => Some(Key::Quote),
      b'<' => Some(Key::Comma),
      b'>' => Some(Key::Period),
      b'?' => Some(Key::Slash),
      b'~' => Some(Key::Backquote),
      _ => None,
    };
  }
  let lower = name.to_ascii_lowercase();
  if let Some(rest) = lower.strip_prefix('f')
    && let Ok(n) = rest.parse::<u8>()
    && (1..=24).contains(&n)
  {
    return Some(FKEYS[usize::from(n) - 1]);
  }
  match lower.as_str() {
    "enter" => Some(Key::Enter),
    "escape" => Some(Key::Escape),
    "tab" => Some(Key::Tab),
    "backspace" => Some(Key::Backspace),
    "delete" => Some(Key::Delete),
    "space" => Some(Key::Space),
    "up" => Some(Key::ArrowUp),
    "down" => Some(Key::ArrowDown),
    "left" => Some(Key::ArrowLeft),
    "right" => Some(Key::ArrowRight),
    "home" => Some(Key::Home),
    "end" => Some(Key::End),
    "pageup" => Some(Key::PageUp),
    "pagedown" => Some(Key::PageDown),
    "insert" => Some(Key::Insert),
    _ => None,
  }
}

#[cfg(test)]
mod tests {
  use std::sync::atomic::{AtomicBool, Ordering};
  use std::sync::{Arc, Mutex};
  use std::thread;
  use std::time::{Duration, Instant};

  struct ReleaseWake(Arc<AtomicBool>);

  impl Drop for ReleaseWake {
    fn drop(&mut self) {
      self.0.store(true, Ordering::Release);
    }
  }

  use libghostty_vt::key::Key;
  use libghostty_vt::style::{Palette, PaletteIndex};

  use super::{KeyInput, KeyMods, PaneCommand, PaneHandle, PtyWriter, key_from_name};
  use crate::terminal::snapshot::{PaneSnapshot, Rgb};

  fn noop_writer() -> PtyWriter {
    Box::new(|_| {})
  }

  fn wait_snapshot(handle: &PaneHandle, pred: impl Fn(&PaneSnapshot) -> bool) -> Arc<PaneSnapshot> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if let Some(snap) = handle.snapshot()
        && pred(&snap)
      {
        return snap;
      }
      thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for snapshot");
  }

  fn wait_bytes(buf: &Mutex<Vec<u8>>, pred: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      let guard = buf.lock().unwrap();
      if pred(&guard) {
        return guard.clone();
      }
      drop(guard);
      thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for pty bytes");
  }

  #[test]
  fn snapshot_from_written_bytes_has_text_and_cursor() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.push_bytes(b"hello\r\n");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.row_text(0), "hello");
    let cursor = snap.cursor.as_ref().expect("cursor");
    assert_eq!((cursor.x, cursor.y), (0, 1));
  }

  #[test]
  fn bell_in_pty_bytes_flags_the_next_snapshot() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.push_bytes(b"hi\x07");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0 && snap.bell);
    assert!(snap.bell);
    let seq = snap.seq;
    handle.push_bytes(b"x");
    let snap = wait_snapshot(&handle, |snap| snap.seq > seq);
    assert!(!snap.bell);
  }

  #[test]
  fn sgr_colors_and_bold_land_in_cells() {
    let handle = PaneHandle::spawn(20, 2, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.push_bytes(b"\x1b[1;31mred\x1b[0m");
    let snap = wait_snapshot(&handle, |snap| snap.row_text(0).starts_with("red"));
    let red = Palette::default().get(PaletteIndex::RED);
    for x in 0..3 {
      let cell = snap.cell(x, 0).expect("cell");
      assert!(cell.bold, "cell {x} should be bold");
      assert_eq!(cell.fg, Some(Rgb(red.r, red.g, red.b)));
    }
    assert_eq!(snap.row_text(0), "red");
  }

  #[test]
  fn text_writes_utf8_to_pty_without_brackets() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap();
    handle.push_bytes(b"\x1b[?2004h");
    wait_snapshot(&handle, |snap| snap.seq > 0);
    handle.send(PaneCommand::Text("é".into()));
    let got = wait_bytes(&collected, |bytes| {
      bytes.windows("é".len()).any(|w| w == "é".as_bytes())
    });
    assert!(
      got.windows("é".len()).any(|w| w == "é".as_bytes()),
      "expected utf-8 in {got:?}"
    );
    assert!(
      !got.windows(6).any(|w| w == b"\x1b[200~"),
      "text must not use bracketed paste"
    );
  }

  #[test]
  fn paste_writes_to_pty_and_brackets_when_mode_is_on() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap();
    handle.send(PaneCommand::Paste("hi".into()));
    let plain = wait_bytes(&collected, |bytes| bytes == b"hi");
    assert_eq!(plain, b"hi");

    handle.push_bytes(b"\x1b[?2004h");
    wait_snapshot(&handle, |snap| snap.seq > 0);
    collected.lock().unwrap().clear();
    handle.send(PaneCommand::Paste("ab".into()));
    let bracketed = wait_bytes(&collected, |bytes| bytes.starts_with(b"\x1b[200~"));
    assert_eq!(bracketed, b"\x1b[200~ab\x1b[201~");
  }

  #[test]
  fn keys_encode_to_pty_bytes() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap();
    handle.send(PaneCommand::Key(KeyInput {
      key: "a".into(),
      text: Some("a".into()),
      mods: KeyMods::default(),
      press: true,
    }));
    wait_bytes(&collected, |bytes| bytes.contains(&b'a'));
    handle.send(PaneCommand::Key(KeyInput {
      key: "enter".into(),
      text: None,
      mods: KeyMods::default(),
      press: true,
    }));
    let got = wait_bytes(&collected, |bytes| bytes.contains(&b'a') && bytes.contains(&b'\r'));
    assert!(got.contains(&b'a'), "expected 'a' in {got:?}");
    assert!(got.contains(&b'\r'), "expected CR in {got:?}");
  }

  #[test]
  fn key_from_name_covers_named_keys() {
    assert_eq!(key_from_name("a"), Some(Key::A));
    assert_eq!(key_from_name("Z"), Some(Key::Z));
    assert_eq!(key_from_name("0"), Some(Key::Digit0));
    assert_eq!(key_from_name("9"), Some(Key::Digit9));
    assert_eq!(key_from_name("-"), Some(Key::Minus));
    assert_eq!(key_from_name("!"), Some(Key::Digit1));
    assert_eq!(key_from_name("@"), Some(Key::Digit2));
    assert_eq!(key_from_name("#"), Some(Key::Digit3));
    assert_eq!(key_from_name("$"), Some(Key::Digit4));
    assert_eq!(key_from_name("%"), Some(Key::Digit5));
    assert_eq!(key_from_name("^"), Some(Key::Digit6));
    assert_eq!(key_from_name("&"), Some(Key::Digit7));
    assert_eq!(key_from_name("*"), Some(Key::Digit8));
    assert_eq!(key_from_name("("), Some(Key::Digit9));
    assert_eq!(key_from_name(")"), Some(Key::Digit0));
    assert_eq!(key_from_name("_"), Some(Key::Minus));
    assert_eq!(key_from_name("+"), Some(Key::Equal));
    assert_eq!(key_from_name("{"), Some(Key::BracketLeft));
    assert_eq!(key_from_name("}"), Some(Key::BracketRight));
    assert_eq!(key_from_name("|"), Some(Key::Backslash));
    assert_eq!(key_from_name(":"), Some(Key::Semicolon));
    assert_eq!(key_from_name("\""), Some(Key::Quote));
    assert_eq!(key_from_name("<"), Some(Key::Comma));
    assert_eq!(key_from_name(">"), Some(Key::Period));
    assert_eq!(key_from_name("?"), Some(Key::Slash));
    assert_eq!(key_from_name("~"), Some(Key::Backquote));
    assert_eq!(key_from_name("enter"), Some(Key::Enter));
    assert_eq!(key_from_name("escape"), Some(Key::Escape));
    assert_eq!(key_from_name("tab"), Some(Key::Tab));
    assert_eq!(key_from_name("backspace"), Some(Key::Backspace));
    assert_eq!(key_from_name("delete"), Some(Key::Delete));
    assert_eq!(key_from_name("space"), Some(Key::Space));
    assert_eq!(key_from_name("up"), Some(Key::ArrowUp));
    assert_eq!(key_from_name("down"), Some(Key::ArrowDown));
    assert_eq!(key_from_name("left"), Some(Key::ArrowLeft));
    assert_eq!(key_from_name("right"), Some(Key::ArrowRight));
    assert_eq!(key_from_name("home"), Some(Key::Home));
    assert_eq!(key_from_name("end"), Some(Key::End));
    assert_eq!(key_from_name("pageup"), Some(Key::PageUp));
    assert_eq!(key_from_name("pagedown"), Some(Key::PageDown));
    assert_eq!(key_from_name("insert"), Some(Key::Insert));
    assert_eq!(key_from_name("f1"), Some(Key::F1));
    assert_eq!(key_from_name("f24"), Some(Key::F24));
    assert_eq!(key_from_name("not-a-key"), None);
  }

  #[test]
  fn zero_sized_spawn_clamps_to_one() {
    let handle = PaneHandle::spawn(0, 0, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.push_bytes(b"x");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.cols, 1);
    assert_eq!(snap.rows, 1);
    assert_eq!(snap.row_text(0), "x");
  }

  #[test]
  fn zero_sized_resize_clamps_to_one() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.send(PaneCommand::Resize {
      cols: 0,
      rows: 0,
      cell_w: 8,
      cell_h: 16,
    });
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.cols, 1);
    assert_eq!(snap.rows, 1);
  }

  #[test]
  fn named_control_key_drops_control_character_text() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap();
    handle.send(PaneCommand::Key(KeyInput {
      key: "enter".into(),
      text: Some("\r".into()),
      mods: KeyMods::default(),
      press: true,
    }));
    let got = wait_bytes(&collected, |bytes| bytes.contains(&b'\r'));
    assert!(got.contains(&b'\r'), "expected CR from enter, got {got:?}");
  }

  #[test]
  fn push_bytes_and_send_never_block() {
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let entered_flag = Arc::clone(&entered);
    let release_flag = Arc::clone(&release);
    let wake = Box::new(move || {
      entered_flag.store(true, Ordering::Release);
      while !release_flag.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(1));
      }
    });
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), wake).unwrap();
    let unblock = ReleaseWake(Arc::clone(&release));
    handle.push_bytes(b"x");
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if entered.load(Ordering::Acquire) {
        break;
      }
      thread::sleep(Duration::from_millis(1));
    }
    assert!(entered.load(Ordering::Acquire), "wake did not run");
    let start = Instant::now();
    handle.push_bytes(b"y");
    handle.send(PaneCommand::ScrollToBottom);
    let elapsed = start.elapsed();
    drop(unblock);
    assert!(elapsed < Duration::from_millis(200), "push_bytes or send blocked");
  }

  #[test]
  fn set_colors_updates_snapshot_background() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {})).unwrap();
    let mut ansi = [Rgb(0, 0, 0); 16];
    ansi[0] = Rgb(10, 11, 12);
    handle.send(PaneCommand::SetColors {
      foreground: Rgb(1, 2, 3),
      background: Rgb(4, 5, 6),
      cursor: Rgb(7, 8, 9),
      ansi,
    });
    let snap = wait_snapshot(&handle, |snap| snap.background == Rgb(4, 5, 6));
    assert_eq!(snap.foreground, Rgb(1, 2, 3));
    assert_eq!(snap.background, Rgb(4, 5, 6));
    assert_eq!(snap.cursor_color, Some(Rgb(7, 8, 9)));
  }

  #[test]
  fn set_colors_applies_ansi_index_one_to_sgr_red_cells() {
    let handle = PaneHandle::spawn(20, 2, None, noop_writer(), Box::new(|| {})).unwrap();
    let mut ansi = [Rgb(0, 0, 0); 16];
    ansi[1] = Rgb(11, 22, 33);
    handle.send(PaneCommand::SetColors {
      foreground: Rgb(1, 2, 3),
      background: Rgb(4, 5, 6),
      cursor: Rgb(7, 8, 9),
      ansi,
    });
    wait_snapshot(&handle, |snap| snap.background == Rgb(4, 5, 6));
    handle.push_bytes(b"\x1b[31mX");
    let snap = wait_snapshot(&handle, |snap| snap.row_text(0).starts_with('X'));
    let cell = snap.cell(0, 0).expect("cell");
    assert_eq!(cell.fg, Some(Rgb(11, 22, 33)));
  }

  #[test]
  fn osc_title_bel_does_not_bell_but_bare_bel_does() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {})).unwrap();
    handle.push_bytes(b"\x1b]0;title\x07hi");
    let snap = wait_snapshot(&handle, |snap| snap.row_text(0).contains("hi"));
    assert!(!snap.bell, "BEL that terminates OSC must not flash");
    let seq = snap.seq;
    handle.push_bytes(b"\x07");
    let snap = wait_snapshot(&handle, |snap| snap.seq > seq && snap.bell);
    assert!(snap.bell);
  }

  #[test]
  fn dsr_request_writes_to_the_pty() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap();
    handle.push_bytes(b"\x1b[6n");
    let got = wait_bytes(&collected, |bytes| !bytes.is_empty());
    assert!(
      got.starts_with(b"\x1b[") && got.ends_with(b"R"),
      "expected DSR cursor report, got {got:?}"
    );
  }

  #[test]
  fn drop_returns_while_the_pane_thread_is_blocked() {
    let entered = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let entered_flag = Arc::clone(&entered);
    let release_flag = Arc::clone(&release);
    let wake = Box::new(move || {
      entered_flag.store(true, Ordering::Release);
      while !release_flag.load(Ordering::Acquire) {
        thread::sleep(Duration::from_millis(1));
      }
    });
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), wake).unwrap();
    let unblock = ReleaseWake(Arc::clone(&release));
    handle.push_bytes(b"x");
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if entered.load(Ordering::Acquire) {
        break;
      }
      thread::sleep(Duration::from_millis(1));
    }
    assert!(entered.load(Ordering::Acquire), "wake did not run");
    let start = Instant::now();
    drop(handle);
    let elapsed = start.elapsed();
    drop(unblock);
    assert!(
      elapsed < Duration::from_millis(200),
      "Drop joined a blocked pane thread"
    );
  }
}
