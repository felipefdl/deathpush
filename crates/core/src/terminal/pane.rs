//! One OS thread per terminal pane. libghostty types stay here; the app sees snapshots.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::sync::{Arc, Mutex, Once};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use libghostty_vt::key::{self, Encoder as KeyEncoder, Event as KeyEvent};
use libghostty_vt::kitty::graphics::{ImageFormat, RustPngDecoder, set_png_decoder};
use libghostty_vt::mouse::{self, Encoder as MouseEncoder, Event as MouseEvent};
use libghostty_vt::paste;
use libghostty_vt::render::{
  CellIterator, Colors, CursorVisualStyle, Dirty, ImageIterator, ImageLayer as VtImageLayer, ImageProtocol,
  RowIteration, RowIterator, Snapshot,
};
use libghostty_vt::screen::CellWide;
use libghostty_vt::search::{MatchBuffer, Search};
use libghostty_vt::selection::Selection;
use libghostty_vt::style::{PaletteIndex, RgbColor, Underline};
use libghostty_vt::terminal::{
  ClipboardMime, ClipboardRead, ClipboardReadError, ClipboardReplyContent, Mode, PointSpace, ScrollViewport,
};
use libghostty_vt::{RenderState, Terminal};

use crate::error::{Error, Result};
use crate::terminal::snapshot::{
  CursorShape, CursorSnapshot, ImageLayer, ImagePixels, PaneSnapshot, PointerShape, Rgb, SearchSnapshot, SearchSpan,
  SnapshotCell, SnapshotImage, SnapshotRow,
};

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
  /// Clipboard text written to the PTY. Unsafe control bytes become spaces; the text is bracketed
  /// when the terminal has paste bracketing on, and its newlines become carriage returns otherwise.
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
  /// Set the search needle (ASCII case-insensitive) over scrollback and screen. A new needle selects
  /// the newest match and scrolls to it; an empty needle ends the search and its highlights.
  SearchQuery(String),
  /// Select the next match toward newer output (`forward`, Enter) or older output (Shift+Enter),
  /// wrapping at the ends, and scroll to it.
  SearchStep {
    /// Toward newer output when true.
    forward: bool,
  },
  /// Stop the pane thread.
  Shutdown,
}

enum CommandEffect {
  Shutdown,
  Dirty,
  /// Dirty, and the user is waiting on it: show live content even during a render hold.
  Live,
  Clean,
}

enum PaneMsg {
  Bytes(Vec<u8>),
  Command(PaneCommand),
}

const BATCH_MESSAGE_LIMIT: usize = 256;
const BATCH_BYTE_LIMIT: usize = 64 * 1024;

/// How long a program may hold the screen with synchronized output (mode 2026) before the pane
/// shows live content again.
const RENDER_HOLD_TIMEOUT: Duration = Duration::from_secs(1);

/// Bytes of Kitty graphics images one screen of a pane may keep, the default Ghostty ships with.
/// The limit is per screen, main and alternate each get one.
const KITTY_IMAGE_STORAGE_LIMIT: u64 = 320 * 1024 * 1024;

/// The shortest wait between two animation frames the pane schedules, about one 120 Hz refresh.
const MIN_ANIMATION_STEP_MS: u64 = 8;

/// Cell size in pixels a pane starts with until the app reports the real one.
const DEFAULT_CELL_PX: (u32, u32) = (8, 16);

/// What one batch of messages did to the terminal.
#[derive(Default)]
struct Batch {
  bytes: usize,
  dirty: bool,
  live: bool,
}

/// The render state and its iterators. The render-hold callback shares them, so a hold can
/// capture the frame it freezes.
struct Frames {
  render: RenderState<'static>,
  rows: RowIterator<'static>,
  cells: CellIterator<'static>,
  images: ImageIterator<'static>,
  /// The rows and colors of the last capture; clean rows are shared with the next one.
  prev: Option<PrevFrame>,
  /// The image list of the last capture and the pixel buffers it holds.
  image_cache: ImageCache,
  /// Cell size in pixels that libghostty sizes images with.
  cell_px: (u32, u32),
}

impl Frames {
  fn new() -> Result<Self> {
    Ok(Self {
      render: RenderState::new().map_err(vt_err)?,
      rows: RowIterator::new().map_err(vt_err)?,
      cells: CellIterator::new().map_err(vt_err)?,
      images: ImageIterator::new().map_err(vt_err)?,
      prev: None,
      image_cache: ImageCache::default(),
      cell_px: DEFAULT_CELL_PX,
    })
  }
}

/// What the last capture read from the render state's image list.
#[derive(Default)]
struct ImageCache {
  /// The render state's placements generation and the cell size the list was read at. `None`
  /// forces the next capture to read the list again.
  read_at: Option<(u64, (u32, u32))>,
  /// Placements in draw order. Handed to every snapshot until the list changes.
  list: Arc<Vec<SnapshotImage>>,
  /// Pixel buffers of the images in `list`, keyed by image id and generation.
  pixels: HashMap<(u32, u64), Arc<ImagePixels>>,
}

struct PrevFrame {
  cols: u16,
  rows: u16,
  colors: Colors,
  lines: Vec<Arc<SnapshotRow>>,
}

/// A program's request to read the clipboard (OSC 52). Answer with [`Self::allow`] or
/// [`Self::deny`]; dropping it denies. The program waits on the answer, so the pane thread
/// waits too, up to [`CLIPBOARD_READ_TIMEOUT`].
#[derive(Debug)]
pub struct ClipboardReadRequest {
  /// Name the program gave itself; empty for OSC 52.
  pub name: String,
  /// The terminal already holds a grant for this program, so no prompt is needed.
  pub granted: bool,
  reply: Sender<Option<String>>,
}

impl ClipboardReadRequest {
  /// Hand the program the clipboard text.
  pub fn allow(self, text: String) {
    let _ = self.reply.send(Some(text));
  }

  /// Answer the program with an empty clipboard.
  pub fn deny(self) {
    let _ = self.reply.send(None);
  }
}

/// Called on the pane thread for every clipboard read that needs a decision.
pub type ClipboardReadHandler = Box<dyn Fn(ClipboardReadRequest) + Send + 'static>;

/// No answer within this long counts as a denial.
const CLIPBOARD_READ_TIMEOUT: Duration = Duration::from_secs(30);

/// A search over the terminal's scrollback and screen.
struct SearchSession {
  search: Search<'static, 'static>,
  needle: String,
  storage: MatchBuffer,
}

/// A synchronized-output hold in progress.
struct RenderHold {
  started: Instant,
  /// The frame on screen when the hold began, until it is published.
  frame: Option<PaneSnapshot>,
}

/// What a batch publishes while a program may be holding the screen.
enum Publish {
  /// The hold is on and its frame is already out.
  Nothing,
  /// The frame the hold froze.
  Held(PaneSnapshot),
  /// Current terminal content.
  Live,
}

struct PaneIo {
  slot: Arc<Mutex<Option<Arc<PaneSnapshot>>>>,
  mouse_tracking: Arc<AtomicBool>,
  shutdown: Arc<AtomicBool>,
  clipboard: ClipboardReadHandler,
}

/// Bytes the pane thread wants written to the PTY (encoded keys, mouse reports, pasted text).
pub type PtyWriter = Box<dyn Fn(Vec<u8>) + Send + 'static>;

/// Handle to a pane thread. Dropping it shuts the thread down and detaches the join handle.
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
  /// It must not block and must hold only weak or lifetime-safe app state. `wake` may still be in
  /// flight briefly after [`Drop`]. Shutdown is an atomic flag plus a [`PaneCommand::Shutdown`] on
  /// the same FIFO; [`Drop`] sets the flag, sends, and detaches the pane thread. The pane thread
  /// owns its snapshot Arcs and exits on the shutdown flag.
  ///
  /// `clipboard` decides OSC 52 clipboard reads on the pane thread; it must hand the request to
  /// whoever can answer and return at once.
  pub fn spawn(
    cols: u16,
    rows: u16,
    scrollback_bytes: Option<usize>,
    writer: PtyWriter,
    wake: Box<dyn Fn() + Send + 'static>,
    clipboard: ClipboardReadHandler,
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
      clipboard,
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

/// Sets the shutdown flag and sends [`PaneCommand::Shutdown`]. The join handle is detached so Drop
/// does not block. The pane thread owns its Arcs and exits on the shutdown flag.
impl Drop for PaneHandle {
  fn drop(&mut self) {
    self.shutdown.store(true, Ordering::Release);
    let _ = self.tx.send(PaneMsg::Command(PaneCommand::Shutdown));
    drop(self.join.take());
  }
}

struct PaneThread {
  terminal: Terminal<'static, 'static>,
  frames: Rc<RefCell<Frames>>,
  hold: Rc<RefCell<Option<RenderHold>>>,
  key_encoder: KeyEncoder<'static>,
  key_event: KeyEvent<'static>,
  mouse_encoder: MouseEncoder<'static>,
  mouse_event: MouseEvent<'static>,
  writer: Rc<PtyWriter>,
  wake: Box<dyn Fn() + Send + 'static>,
  slot: Arc<Mutex<Option<Arc<PaneSnapshot>>>>,
  search: Option<SearchSession>,
  last_search: Option<SearchSnapshot>,
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
  /// The clock handed to libghostty's animation ticks: milliseconds since this instant.
  epoch: Instant,
  /// When the next animation frame is due, if one is playing.
  next_animation: Option<Instant>,
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
    install_png_decoder();
    let PaneIo {
      slot,
      mouse_tracking,
      shutdown,
      clipboard,
    } = io;
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
    {
      let shutdown = Arc::clone(&shutdown);
      terminal
        .on_clipboard_read(move |_term, request| answer_clipboard_read(&clipboard, &shutdown, request))
        .map_err(vt_err)?;
    }
    let frames = Rc::new(RefCell::new(Frames::new()?));
    let hold = Rc::new(RefCell::new(None));
    {
      let frames = Rc::clone(&frames);
      let hold = Rc::clone(&hold);
      terminal
        .on_render_hold(move |term, held| {
          *hold.borrow_mut() = held.then(|| RenderHold {
            started: Instant::now(),
            // A failed capture leaves the last published frame on screen.
            frame: frames
              .try_borrow_mut()
              .ok()
              .and_then(|mut frames| capture(&mut frames, term).ok()),
          });
        })
        .map_err(vt_err)?;
    }
    if let Err(err) = terminal.set_terminfo_name(crate::pty::TERM_NAME) {
      tracing::warn!(%err, "set terminfo name failed");
    }
    configure_kitty_graphics(&mut terminal);
    let (cell_w, cell_h) = DEFAULT_CELL_PX;
    if let Err(err) = terminal.resize(cols, rows, cell_w, cell_h) {
      tracing::warn!(%err, "terminal pixel resize failed");
    }
    if let Err(err) = terminal.set_scrollback_max_bytes(scrollback_bytes) {
      tracing::warn!(%err, "set scrollback failed");
    }
    Ok(Self {
      terminal,
      frames,
      hold,
      key_encoder: KeyEncoder::new().map_err(vt_err)?,
      key_event: KeyEvent::new().map_err(vt_err)?,
      mouse_encoder: MouseEncoder::new().map_err(vt_err)?,
      mouse_event: MouseEvent::new().map_err(vt_err)?,
      writer,
      wake,
      slot,
      search: None,
      last_search: None,
      mouse_tracking,
      seq: 0,
      cols,
      rows,
      cell_w,
      cell_h,
      pty_buf: Vec::new(),
      last_snapshot_error: None,
      shutdown,
      bell,
      epoch: Instant::now(),
      next_animation: None,
    })
  }

  fn shutting_down(&self) -> bool {
    self.shutdown.load(Ordering::Acquire)
  }

  fn apply_msg(&mut self, msg: PaneMsg, batch: &mut Batch) -> bool {
    match msg {
      PaneMsg::Bytes(data) => {
        batch.bytes = batch.bytes.saturating_add(data.len());
        self.terminal.vt_write(&data);
        batch.dirty = true;
        false
      }
      PaneMsg::Command(command) => match self.apply(command) {
        CommandEffect::Shutdown => true,
        CommandEffect::Dirty => {
          batch.dirty = true;
          false
        }
        CommandEffect::Live => {
          batch.dirty = true;
          batch.live = true;
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
      let first = match self.wake_deadline() {
        None => match rx.recv() {
          Ok(msg) => msg,
          Err(_) => break,
        },
        Some(deadline) => match rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
          Ok(msg) => msg,
          Err(RecvTimeoutError::Timeout) => {
            self.on_timer();
            continue;
          }
          Err(RecvTimeoutError::Disconnected) => break,
        },
      };
      if self.shutting_down() {
        break;
      }
      let mut messages = 1usize;
      let mut batch = Batch::default();
      if self.apply_msg(first, &mut batch) {
        break;
      }
      while messages < BATCH_MESSAGE_LIMIT && batch.bytes < BATCH_BYTE_LIMIT {
        if self.shutting_down() {
          return;
        }
        match rx.try_recv() {
          Ok(msg) => {
            messages += 1;
            if self.apply_msg(msg, &mut batch) {
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
      if batch.dirty {
        self.update_mouse_tracking();
        self.publish(batch.live);
      }
    }
  }

  fn hold_deadline(&self) -> Option<Instant> {
    self
      .hold
      .borrow()
      .as_ref()
      .map(|hold| hold.started + RENDER_HOLD_TIMEOUT)
  }

  /// The earlier of the hold timeout and the next animation frame.
  fn wake_deadline(&self) -> Option<Instant> {
    match (self.hold_deadline(), self.next_animation) {
      (Some(hold), Some(frame)) => Some(hold.min(frame)),
      (hold, frame) => hold.or(frame),
    }
  }

  /// A wake with no message: a hold that was never released or an animation frame that is due.
  fn on_timer(&mut self) {
    let now = Instant::now();
    if self.hold_deadline().is_some_and(|deadline| deadline <= now) {
      // The program never released its hold: give up on it and show what it wrote.
      self.publish(false);
    } else if self.next_animation.is_some_and(|due| due <= now) {
      self.next_animation = None;
      // A held screen shows its frozen frame, so the frame waits; the publish that ends the hold
      // ticks again.
      if self.hold.borrow().is_none() {
        self.publish(false);
      }
    }
  }

  /// Advance Kitty animations to now and schedule the wake for the next frame. libghostty keeps no
  /// timer: it answers with the time the next frame is due on the clock it was given.
  fn tick_animation(&mut self) {
    let now_ms = u64::try_from(self.epoch.elapsed().as_millis()).unwrap_or(u64::MAX);
    self.next_animation = match self.terminal.kitty_graphics_animation_tick(now_ms) {
      Ok(next) => {
        next.map(|due_ms| self.epoch + Duration::from_millis(due_ms.max(now_ms.saturating_add(MIN_ANIMATION_STEP_MS))))
      }
      Err(err) => {
        tracing::warn!(%err, "kitty animation tick failed");
        None
      }
    };
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
        self.frames.borrow_mut().cell_px = (self.cell_w, self.cell_h);
        if let Err(err) = self.terminal.resize(cols, rows, self.cell_w, self.cell_h) {
          tracing::warn!(%err, "terminal resize failed");
        }
        CommandEffect::Dirty
      }
      PaneCommand::Scroll(delta) => {
        self.terminal.scroll_viewport(ScrollViewport::Delta(delta));
        CommandEffect::Live
      }
      PaneCommand::ScrollToBottom => {
        self.terminal.scroll_viewport(ScrollViewport::Bottom);
        CommandEffect::Live
      }
      PaneCommand::SetScrollbackBytes(bytes) => {
        if let Err(err) = self.terminal.set_scrollback_max_bytes(bytes) {
          tracing::warn!(%err, "set scrollback failed");
        }
        CommandEffect::Dirty
      }
      PaneCommand::SearchQuery(needle) => {
        self.set_search_query(needle);
        CommandEffect::Live
      }
      PaneCommand::SearchStep { forward } => {
        self.step_search(forward);
        CommandEffect::Live
      }
      PaneCommand::SetColors {
        foreground,
        background,
        cursor,
        ansi,
      } => {
        self.apply_theme_colors(foreground, background, cursor, ansi);
        CommandEffect::Live
      }
    }
  }

  fn set_search_query(&mut self, needle: String) {
    if needle.is_empty() {
      self.search = None;
      self.last_search = None;
      return;
    }
    if self.search.is_none() {
      match Search::new(&mut self.terminal) {
        Ok(search) => {
          self.search = Some(SearchSession {
            search,
            needle: String::new(),
            storage: MatchBuffer::new(),
          });
        }
        Err(err) => {
          tracing::warn!(%err, "terminal search start failed");
          return;
        }
      }
    }
    let Some(session) = self.search.as_mut() else {
      return;
    };
    if session.needle == needle {
      return;
    }
    let started = session
      .search
      .set_needle(&mut self.terminal, needle.as_bytes())
      .and_then(|search| search.run(&mut self.terminal))
      .and_then(|()| session.search.select_next(&mut self.terminal).map(|_| ()));
    if let Err(err) = started {
      tracing::warn!(%err, "terminal search failed");
    }
    session.needle = needle;
  }

  /// libghostty orders matches newest first, so the newer neighbor is its "previous" match.
  fn step_search(&mut self, forward: bool) {
    let Some(session) = self.search.as_mut() else {
      return;
    };
    let moved = if forward {
      session.search.select_prev(&mut self.terminal)
    } else {
      session.search.select_next(&mut self.terminal)
    };
    if let Err(err) = moved {
      tracing::warn!(%err, "terminal search step failed");
    }
  }

  fn search_results(&mut self) -> Option<SearchSnapshot> {
    let session = self.search.as_mut()?;
    let results = match collect_search(session, &mut self.terminal, self.cols, self.rows) {
      Ok(results) => Some(results),
      Err(err) => {
        tracing::warn!(%err, "terminal search read failed");
        None
      }
    };
    self.last_search.clone_from(&results);
    results
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

  /// The terminal frames the text for its current modes. Pastes are never refused: there is no
  /// confirmation prompt, so text that could run a command goes through as it always has.
  fn handle_paste(&mut self, text: String) {
    let mimes = [ClipboardMime::new("text/plain")];
    let options = paste::Options::new().with_allow_unsafe(true);
    if let Err(err) = self
      .terminal
      .paste(options, &mimes, |_, out| out.write_all(text.as_bytes()))
    {
      tracing::warn!(%err, "paste failed");
    }
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

  /// `live` shows current content even while a program holds the screen.
  fn publish(&mut self, live: bool) {
    let snapshot = match self.next_publish(live) {
      Publish::Nothing => return,
      Publish::Held(frame) => Ok((frame, false)),
      Publish::Live => {
        self.tick_animation();
        capture(&mut self.frames.borrow_mut(), &self.terminal).map(|frame| (frame, true))
      }
    };
    match snapshot {
      Ok((mut snapshot, current)) => {
        self.last_snapshot_error = None;
        self.seq += 1;
        snapshot.seq = self.seq;
        snapshot.bell = self.bell.replace(false);
        // A held frame is older than the terminal, so it keeps the highlights it was drawn with.
        snapshot.search = if current {
          self.search_results()
        } else {
          self.last_search.clone()
        };
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

  /// During a synchronized-output hold the pane shows the frame from when the hold began, once,
  /// then nothing new until the program releases it or [`RENDER_HOLD_TIMEOUT`] passes.
  fn next_publish(&mut self, live: bool) -> Publish {
    let mut hold = self.hold.borrow_mut();
    let Some(active) = hold.as_mut() else {
      return Publish::Live;
    };
    if active.started.elapsed() >= RENDER_HOLD_TIMEOUT {
      *hold = None;
      drop(hold);
      if let Err(err) = self.terminal.set_mode(Mode::SYNC_OUTPUT, false) {
        tracing::warn!(%err, "ending synchronized output failed");
      }
      return Publish::Live;
    }
    if live {
      return Publish::Live;
    }
    match active.frame.take() {
      Some(frame) => Publish::Held(frame),
      None => Publish::Nothing,
    }
  }
}

/// The terminal's current viewport as a snapshot. `seq`, `bell`, and `search` are left for the
/// publisher.
///
/// Rows libghostty reports clean are shared with the previous capture. Every capture is relative
/// to the one before it, published or not, because it clears the render state's dirty flags.
fn capture(frames: &mut Frames, terminal: &Terminal<'static, 'static>) -> Result<PaneSnapshot> {
  let Frames {
    render,
    rows,
    cells,
    images,
    prev,
    image_cache,
    cell_px,
  } = frames;
  // A failed capture leaves the dirty flags set and no baseline, so the next one rebuilds every row.
  let old = prev.take();
  let snap = render.update(terminal).map_err(vt_err)?;
  let cols = snap.cols().map_err(vt_err)?;
  let row_count = snap.rows().map_err(vt_err)?;
  let colors = snap.colors().map_err(vt_err)?;
  let cursor_visible = snap.cursor_visible().map_err(vt_err)?;
  let cursor_blinking = snap.cursor_blinking().map_err(vt_err)?;
  let cursor_style = snap.cursor_visual_style().map_err(vt_err)?;
  let cursor_viewport = snap.cursor_viewport().map_err(vt_err)?;
  let cursor_color = snap.cursor_color().map_err(vt_err)?;
  let dirty = snap.dirty().map_err(vt_err)?;
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
  let base = old.filter(|old| {
    dirty != Dirty::Full
      && old.cols == cols
      && old.rows == row_count
      && old.colors == colors
      && old.lines.len() == usize::from(row_count)
  });
  let mut row_iter = rows.update(&snap).map_err(vt_err)?;
  let lines = match base {
    Some(base) => {
      let mut lines = base.lines;
      while let Some((y, row)) = row_iter.next_dirty() {
        let Some(slot) = lines.get_mut(usize::from(y)) else {
          return Err(Error::Other(format!("dirty row {y} outside {row_count} rows")));
        };
        *slot = Arc::new(build_row(cells, row, default_fg, default_bg)?);
      }
      lines
    }
    None => {
      let mut lines = Vec::with_capacity(usize::from(row_count));
      while let Some(row) = row_iter.next() {
        lines.push(Arc::new(build_row(cells, row, default_fg, default_bg)?));
      }
      lines
    }
  };
  if lines.len() != usize::from(row_count) {
    return Err(Error::Other(format!("captured {} of {row_count} rows", lines.len())));
  }
  let image_list = match capture_images(image_cache, images, &snap, terminal, *cell_px) {
    Ok(list) => list,
    // The text still draws; the images come back with the next change.
    Err(err) => {
      tracing::warn!(%err, "kitty image capture failed");
      image_cache.read_at = None;
      Arc::clone(&image_cache.list)
    }
  };
  snap.clean().map_err(vt_err)?;
  let bar = terminal.scrollbar().map_err(vt_err)?;
  let viewport_offset = bar.total.saturating_sub(bar.offset.saturating_add(bar.len)) as usize;
  let scrollback_rows = terminal.scrollback_rows().map_err(vt_err)?;
  let pointer = terminal.mouse_shape().map(pointer_shape).unwrap_or_default();
  *prev = Some(PrevFrame {
    cols,
    rows: row_count,
    colors,
    lines: lines.clone(),
  });
  Ok(PaneSnapshot {
    seq: 0,
    cols,
    rows: row_count,
    lines,
    cursor,
    background: default_bg,
    foreground: default_fg,
    cursor_color: cursor_color.map(|color| Rgb(color.r, color.g, color.b)),
    viewport_offset,
    scrollback_rows,
    bell: false,
    pointer,
    search: None,
    cell_width: cell_px.0,
    cell_height: cell_px.1,
    images: image_list,
  })
}

/// libghostty's PNG hook is process-global and unsynchronized, so it is installed once, by the
/// first pane, before that pane's terminal exists. It decodes with libghostty's own `png`-crate
/// decoder, which allocates the pixels through the allocator libghostty hands it and so stays inside
/// the image size limits libghostty enforces on untrusted headers.
fn install_png_decoder() {
  static INSTALL: Once = Once::new();
  INSTALL.call_once(|| {
    // SAFETY: no pane thread has a terminal until the first pane gets here, and the Once keeps
    // every other pane thread from starting its terminal before the decoder is in.
    if let Err(err) = unsafe { set_png_decoder(Some(Box::new(RustPngDecoder::new()))) } {
      tracing::warn!(%err, "installing the kitty PNG decoder failed");
    }
  });
}

/// Turn on Kitty graphics for a terminal: images sent inline in the escape stream only. The file,
/// temporary-file, and shared-memory mediums make the terminal read the local machine on a
/// program's say-so, which a program running over ssh or in an untrusted repository must not get.
fn configure_kitty_graphics(terminal: &mut Terminal<'static, 'static>) {
  let configured = terminal
    .set_kitty_image_from_file_allowed(false)
    .and_then(|terminal| terminal.set_kitty_image_temp_file_dir(None))
    .and_then(|terminal| terminal.set_kitty_image_from_shared_mem_allowed(false))
    .and_then(|terminal| terminal.set_kitty_image_storage_limit(KITTY_IMAGE_STORAGE_LIMIT));
  if let Err(err) = configured {
    tracing::warn!(%err, "enabling kitty graphics failed");
  }
}

/// The render state's image list as snapshot placements in draw order.
///
/// Reads the list only when libghostty's placements generation (or the cell size) changed since the
/// last read; otherwise hands back the same `Arc`. Pixel buffers are shared by (image id, image
/// generation) with the previous list, so an image that did not change is never copied again, and
/// one that left the list is dropped with it.
fn capture_images(
  cache: &mut ImageCache,
  images: &mut ImageIterator<'static>,
  snap: &Snapshot<'static, '_>,
  terminal: &Terminal<'static, 'static>,
  cell_px: (u32, u32),
) -> Result<Arc<Vec<SnapshotImage>>> {
  let generation = snap.image_placements_generation().map_err(vt_err)?;
  if cache.read_at == Some((generation, cell_px)) {
    return Ok(Arc::clone(&cache.list));
  }
  let mut list = Vec::new();
  let mut pixels: HashMap<(u32, u64), Arc<ImagePixels>> = HashMap::new();
  let mut complete = true;
  let mut placements = images.update(snap).map_err(vt_err)?;
  while let Some(placement) = placements.next() {
    if !matches!(placement.protocol(), Ok(ImageProtocol::Kitty)) {
      continue;
    }
    let id = placement.image_id().map_err(vt_err)?;
    let image_generation = placement.image_generation().map_err(vt_err)?;
    let key = (id, image_generation);
    let shared = match pixels.get(&key).or_else(|| cache.pixels.get(&key)) {
      Some(shared) => Arc::clone(shared),
      None => match read_pixels(terminal, id, image_generation) {
        Some(read) => Arc::new(read),
        // The pixels are not readable yet: draw nothing for it and look again next time.
        None => {
          complete = false;
          continue;
        }
      },
    };
    pixels.insert(key, Arc::clone(&shared));
    let layer = match placement.layer().map_err(vt_err)? {
      VtImageLayer::BelowBackground => ImageLayer::BelowBackground,
      VtImageLayer::BelowText => ImageLayer::BelowText,
      VtImageLayer::AboveText => ImageLayer::AboveText,
    };
    list.push(SnapshotImage {
      pixels: shared,
      layer,
      z: placement.z().map_err(vt_err)?,
      col: placement.viewport_col().map_err(vt_err)?,
      row: placement.viewport_row().map_err(vt_err)?,
      grid_cols: placement.grid_cols().map_err(vt_err)?,
      grid_rows: placement.grid_rows().map_err(vt_err)?,
      offset_x: placement.cell_offset_x().map_err(vt_err)?,
      offset_y: placement.cell_offset_y().map_err(vt_err)?,
      width: placement.pixel_width().map_err(vt_err)?,
      height: placement.pixel_height().map_err(vt_err)?,
      source_x: placement.source_x().map_err(vt_err)?,
      source_y: placement.source_y().map_err(vt_err)?,
      source_width: placement.source_width().map_err(vt_err)?,
      source_height: placement.source_height().map_err(vt_err)?,
    });
  }
  // Iteration order is not draw order. The sort is stable, so equal layers and z keep their order.
  list.sort_by_key(|image| (image.layer, image.z));
  cache.read_at = complete.then_some((generation, cell_px));
  cache.pixels = pixels;
  // A new generation does not always mean a new list (a rewritten placeholder row, say): keep the
  // old `Arc` then so readers can tell by pointer that nothing changed.
  if *cache.list != list {
    cache.list = Arc::new(list);
  }
  Ok(Arc::clone(&cache.list))
}

/// The decoded pixels of one Kitty image as RGBA, or `None` when they are not there yet or not in
/// a shape the pane can hand on.
fn read_pixels(terminal: &Terminal<'static, 'static>, id: u32, generation: u64) -> Option<ImagePixels> {
  let graphics = terminal.kitty_graphics().ok()?;
  let image = graphics.image(id)?;
  let data = image.data().ok()??;
  let (width, height) = (image.width().ok()?, image.height().ok()?);
  let pixel_count = (width as usize).checked_mul(height as usize)?;
  let format = image.format().ok()?;
  let channels = match format {
    ImageFormat::Rgba => 4,
    ImageFormat::Rgb => 3,
    ImageFormat::GrayAlpha => 2,
    ImageFormat::Gray => 1,
    // libghostty decodes PNG at transmission; anything else is not a format this pane knows.
    _ => return None,
  };
  if data.len() != pixel_count.checked_mul(channels)? || pixel_count == 0 {
    return None;
  }
  let rgba = match format {
    ImageFormat::Rgb => data
      .as_chunks::<3>()
      .0
      .iter()
      .flat_map(|&[r, g, b]| [r, g, b, 255])
      .collect(),
    ImageFormat::GrayAlpha => data
      .as_chunks::<2>()
      .0
      .iter()
      .flat_map(|&[l, a]| [l, l, l, a])
      .collect(),
    ImageFormat::Gray => data.iter().flat_map(|&g| [g, g, g, 255]).collect(),
    _ => data.to_vec(),
  };
  Some(ImagePixels {
    id,
    generation,
    width,
    height,
    rgba,
  })
}

fn build_row(
  cells: &mut CellIterator<'static>,
  row: &RowIteration<'static, '_>,
  default_fg: Rgb,
  default_bg: Rgb,
) -> Result<SnapshotRow> {
  let mut out = Vec::new();
  let mut cell_iter = cells.update(row).map_err(vt_err)?;
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
    out.push(SnapshotCell {
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
  Ok(SnapshotRow { cells: out })
}

fn pointer_shape(shape: mouse::Shape) -> PointerShape {
  use mouse::Shape;
  match shape {
    Shape::Default => PointerShape::Default,
    Shape::ContextMenu => PointerShape::ContextMenu,
    Shape::Help => PointerShape::Help,
    Shape::Pointer => PointerShape::Pointer,
    Shape::Progress => PointerShape::Progress,
    Shape::Wait => PointerShape::Wait,
    Shape::Cell => PointerShape::Cell,
    Shape::Crosshair => PointerShape::Crosshair,
    Shape::Text => PointerShape::Text,
    Shape::VerticalText => PointerShape::VerticalText,
    Shape::Alias => PointerShape::Alias,
    Shape::Copy => PointerShape::Copy,
    Shape::Move => PointerShape::Move,
    Shape::NoDrop => PointerShape::NoDrop,
    Shape::NotAllowed => PointerShape::NotAllowed,
    Shape::Grab => PointerShape::Grab,
    Shape::Grabbing => PointerShape::Grabbing,
    Shape::AllScroll => PointerShape::AllScroll,
    Shape::ColResize => PointerShape::ColResize,
    Shape::RowResize => PointerShape::RowResize,
    Shape::NResize => PointerShape::NResize,
    Shape::EResize => PointerShape::EResize,
    Shape::SResize => PointerShape::SResize,
    Shape::WResize => PointerShape::WResize,
    Shape::NeResize => PointerShape::NeResize,
    Shape::NwResize => PointerShape::NwResize,
    Shape::SeResize => PointerShape::SeResize,
    Shape::SwResize => PointerShape::SwResize,
    Shape::EwResize => PointerShape::EwResize,
    Shape::NsResize => PointerShape::NsResize,
    Shape::NeswResize => PointerShape::NeswResize,
    Shape::NwseResize => PointerShape::NwseResize,
    Shape::ZoomIn => PointerShape::ZoomIn,
    Shape::ZoomOut => PointerShape::ZoomOut,
    _ => PointerShape::default(),
  }
}

/// Matches and the selected match of a search, as viewport runs.
fn collect_search(
  session: &mut SearchSession,
  terminal: &mut Terminal<'static, 'static>,
  cols: u16,
  rows: u16,
) -> Result<SearchSnapshot> {
  let SearchSession { search, storage, .. } = session;
  search.run(terminal).map_err(vt_err)?;
  let total = search.total_matches().map_err(vt_err)?;
  let selected = search.selected_index().map_err(vt_err)?;
  let snapshot = search.snapshot(terminal).map_err(vt_err)?;
  let mut out = SearchSnapshot {
    total,
    // libghostty indexes newest first; the find widget counts from the top.
    current: selected.filter(|index| *index < total).map(|index| total - index),
    ..SearchSnapshot::default()
  };
  if let Some(selected) = snapshot.selected_match().map_err(vt_err)? {
    push_spans(snapshot.terminal(), &selected, cols, rows, &mut out.current_match);
  }
  for found in snapshot.viewport_matches(storage).map_err(vt_err)? {
    push_spans(snapshot.terminal(), &found, cols, rows, &mut out.matches);
  }
  Ok(out)
}

/// One run per viewport row the match touches. An end outside the viewport clips to its edge.
fn push_spans(
  terminal: &Terminal<'static, 'static>,
  found: &Selection<'_>,
  cols: u16,
  rows: u16,
  out: &mut Vec<SearchSpan>,
) {
  let at = |grid_ref| {
    terminal
      .point_from_grid_ref(&grid_ref, PointSpace::Viewport)
      .ok()
      .flatten()
  };
  let last_col = cols.saturating_sub(1);
  let last_row = u32::from(rows.saturating_sub(1));
  let (start, end) = match (at(found.start()), at(found.end())) {
    (Some(start), Some(end)) => ((start.x, start.y), (end.x, end.y)),
    (None, Some(end)) => ((0, 0), (end.x, end.y)),
    (Some(start), None) => ((start.x, start.y), (last_col, last_row)),
    (None, None) => return,
  };
  let (start, end) = if (start.1, start.0) <= (end.1, end.0) {
    (start, end)
  } else {
    (end, start)
  };
  for y in start.1..=end.1.min(last_row) {
    out.push(SearchSpan {
      y: y as u16,
      x0: if y == start.1 { start.0.min(last_col) } else { 0 },
      x1: if y == end.1 { end.0.min(last_col) } else { last_col },
    });
  }
}

/// Asks the app for the clipboard and waits for its answer, since the program is waiting on this
/// terminal for the same. Targets-only listings and non-text requests need no decision.
fn answer_clipboard_read(handler: &ClipboardReadHandler, shutdown: &AtomicBool, request: ClipboardRead<'_>) {
  const TEXT: &str = "text/plain";
  let available = [ClipboardMime::new(TEXT)];
  let mut mimes = request.mimes().peekable();
  if mimes.peek().is_none() {
    drop(mimes);
    request.reply(Ok(&[]), &available, false);
    return;
  }
  let wants_text = mimes.any(|mime| mime.eq_ignore_ascii_case(TEXT.as_bytes()));
  if !wants_text {
    request.reply(Err(ClipboardReadError::Unsupported), &[], false);
    return;
  }
  let (reply, answer) = mpsc::channel();
  handler(ClipboardReadRequest {
    name: String::from_utf8_lossy(request.name()).into_owned(),
    granted: request.granted(),
    reply,
  });
  let deadline = Instant::now() + CLIPBOARD_READ_TIMEOUT;
  let text = loop {
    match answer.recv_timeout(Duration::from_millis(50)) {
      Ok(text) => break text,
      Err(RecvTimeoutError::Timeout) if !shutdown.load(Ordering::Acquire) && Instant::now() < deadline => {}
      Err(_) => break None,
    }
  };
  match text {
    Some(text) => request.reply(
      Ok(&[ClipboardReplyContent::new(TEXT, text.as_bytes())]),
      &available,
      false,
    ),
    None => request.reply(Err(ClipboardReadError::Denied), &[], false),
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

  use libghostty_vt::Terminal;

  use super::{
    ClipboardReadHandler, Frames, KeyInput, KeyMods, PaneCommand, PaneHandle, PtyWriter, capture, key_from_name,
  };
  use crate::terminal::snapshot::{ImageLayer, PaneSnapshot, PointerShape, Rgb};

  fn noop_writer() -> PtyWriter {
    Box::new(|_| {})
  }

  fn deny_clipboard() -> ClipboardReadHandler {
    Box::new(|request| request.deny())
  }

  fn collecting_writer() -> (PtyWriter, Arc<Mutex<Vec<u8>>>) {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    (
      Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes)),
      collected,
    )
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
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"hello\r\n");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.row_text(0), "hello");
    let cursor = snap.cursor.as_ref().expect("cursor");
    assert_eq!((cursor.x, cursor.y), (0, 1));
  }

  #[test]
  fn bell_in_pty_bytes_flags_the_next_snapshot() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 2, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
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
  fn paste_frames_text_for_the_terminal_mode() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
    handle.send(PaneCommand::Paste("a\nb".into()));
    let plain = wait_bytes(&collected, |bytes| bytes.len() >= 3);
    assert_eq!(plain, b"a\rb", "unbracketed newlines become carriage returns");

    handle.push_bytes(b"\x1b[?2004h");
    wait_snapshot(&handle, |snap| snap.seq > 0);
    collected.lock().unwrap().clear();
    handle.send(PaneCommand::Paste("a\x1b[201~b".into()));
    let bracketed = wait_bytes(&collected, |bytes| bytes.ends_with(b"b\x1b[201~"));
    assert_eq!(
      bracketed, b"\x1b[200~a [201~b\x1b[201~",
      "pasted text must not close the bracket itself"
    );
  }

  #[test]
  fn synchronized_output_shows_the_finished_frame_only() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"\x1b[?2026hframe one\x1b[?2026l\x1b[?2026h\rframe two");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.row_text(0), "frame one", "a held frame must not show half drawn");
    thread::sleep(Duration::from_millis(200));
    assert_eq!(handle.snapshot().unwrap().row_text(0), "frame one");
    handle.push_bytes(b"\x1b[?2026l");
    wait_snapshot(&handle, |snap| snap.row_text(0) == "frame two");
  }

  #[test]
  fn a_hold_that_is_never_released_times_out() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    let start = Instant::now();
    handle.push_bytes(b"\x1b[?2026hstuck");
    wait_snapshot(&handle, |snap| snap.row_text(0) == "stuck");
    assert!(start.elapsed() >= Duration::from_millis(900), "the hold ended early");
  }

  #[test]
  fn scrolling_shows_live_content_during_a_hold() {
    let handle = PaneHandle::spawn(20, 2, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"one\r\ntwo\r\nthree");
    let snap = wait_snapshot(&handle, |snap| snap.row_text(1) == "three");
    handle.push_bytes(b"\x1b[?2026h");
    let snap = wait_snapshot(&handle, |next| next.seq > snap.seq);
    handle.send(PaneCommand::Scroll(-1));
    let scrolled = wait_snapshot(&handle, |next| next.seq > snap.seq);
    assert_eq!(scrolled.row_text(0), "one");
  }

  #[test]
  fn keys_encode_to_pty_bytes() {
    let collected = Arc::new(Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(0, 0, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"x");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.cols, 1);
    assert_eq!(snap.rows, 1);
    assert_eq!(snap.row_text(0), "x");
  }

  #[test]
  fn zero_sized_resize_clamps_to_one() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), wake, deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 2, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
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
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), wake, deny_clipboard()).unwrap();
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

  struct ExitFlag(Arc<AtomicBool>);

  impl Drop for ExitFlag {
    fn drop(&mut self) {
      self.0.store(true, Ordering::Release);
    }
  }

  #[test]
  fn pane_thread_exits_after_handle_is_dropped() {
    let exited = Arc::new(AtomicBool::new(false));
    let flag = ExitFlag(Arc::clone(&exited));
    let writer: PtyWriter = Box::new(move |_| {
      let _ = &flag;
    });
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), deny_clipboard()).unwrap();
    drop(handle);
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if exited.load(Ordering::Acquire) {
        break;
      }
      thread::sleep(Duration::from_millis(1));
    }
    assert!(exited.load(Ordering::Acquire), "pane thread did not exit after Drop");
  }

  #[test]
  fn osc_22_changes_the_pointer_shape() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"x");
    let snap = wait_snapshot(&handle, |snap| snap.seq > 0);
    assert_eq!(snap.pointer, PointerShape::Text, "programs start with the text pointer");
    handle.push_bytes(b"\x1b]22;pointer\x07");
    wait_snapshot(&handle, |snap| snap.pointer == PointerShape::Pointer);
    handle.push_bytes(b"\x1b]22;crosshair\x1b\\");
    wait_snapshot(&handle, |snap| snap.pointer == PointerShape::Crosshair);
    handle.push_bytes(b"\x1b]22;\x07");
    wait_snapshot(&handle, |snap| snap.pointer == PointerShape::Text);
  }

  fn shared(a: &PaneSnapshot, b: &PaneSnapshot, y: usize) -> bool {
    Arc::ptr_eq(&a.lines[y], &b.lines[y])
  }

  #[test]
  fn writing_one_row_leaves_the_other_rows_shared() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"a\r\nb\r\nc\r\nd");
    let before = wait_snapshot(&handle, |snap| snap.row_text(3) == "d");
    handle.push_bytes(b"e");
    let after = wait_snapshot(&handle, |snap| snap.seq > before.seq);
    assert_eq!(after.row_text(3), "de");
    for y in 0..3 {
      assert!(shared(&before, &after, y), "row {y} was clean and must be reused");
    }
    assert!(!shared(&before, &after, 3), "row 3 changed and must be rebuilt");
    // An unrelated publish (a bell) leaves every row alone.
    handle.push_bytes(b"\x07");
    let bell = wait_snapshot(&handle, |snap| snap.seq > after.seq);
    for y in 0..4 {
      assert!(shared(&after, &bell, y), "row {y} was clean and must be reused");
    }
  }

  #[test]
  fn a_resize_or_new_colors_rebuild_every_row() {
    let handle = PaneHandle::spawn(20, 4, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap();
    handle.push_bytes(b"a\r\nb\r\nc\r\nd");
    let before = wait_snapshot(&handle, |snap| snap.row_text(3) == "d");
    handle.send(PaneCommand::Resize {
      cols: 10,
      rows: 4,
      cell_w: 8,
      cell_h: 16,
    });
    let resized = wait_snapshot(&handle, |snap| snap.cols == 10);
    assert_eq!(resized.row_text(3), "d");
    for y in 0..4 {
      assert!(!shared(&before, &resized, y), "row {y} must be rebuilt after a resize");
    }
    handle.send(PaneCommand::SetColors {
      foreground: Rgb(1, 2, 3),
      background: Rgb(4, 5, 6),
      cursor: Rgb(7, 8, 9),
      ansi: [Rgb(0, 0, 0); 16],
    });
    let recolored = wait_snapshot(&handle, |snap| snap.background == Rgb(4, 5, 6));
    for y in 0..4 {
      assert!(
        !shared(&resized, &recolored, y),
        "row {y} must be rebuilt after new colors"
      );
      assert_eq!(recolored.cell(0, y as u16).unwrap().bg, Some(Rgb(4, 5, 6)));
    }
  }

  /// Rebuilding only the dirty rows must show exactly what a rebuild of every row shows.
  #[test]
  fn dirty_row_capture_matches_a_full_rebuild() {
    let mut terminal = Terminal::new(12, 5).unwrap();
    let mut frames = Frames::new().unwrap();
    let mut check = |terminal: &Terminal<'static, 'static>, step: &str| {
      let incremental = capture(&mut frames, terminal).unwrap();
      frames.prev = None;
      let full = capture(&mut frames, terminal).unwrap();
      assert_eq!(incremental.lines, full.lines, "after {step}");
      assert_eq!(incremental.cursor, full.cursor, "cursor after {step}");
    };
    check(&terminal, "start");
    let flood = "line\r\n".repeat(40);
    let steps: [(&str, &[u8]); 12] = [
      ("text", b"one\r\ntwo\r\nthree"),
      ("sgr", b"\x1b[1;32mgreen\x1b[0m plain"),
      ("cursor jump", b"\x1b[1;1HX\x1b[5;3HY"),
      ("erase line", b"\x1b[2;1H\x1b[2K"),
      ("scroll", b"\r\n\r\n\r\n\r\n\r\nafter scroll"),
      ("insert lines", b"\x1b[2;1H\x1b[2L"),
      ("wide", "\x1b[3;1H中文 ok".as_bytes()),
      ("scroll region", b"\x1b[2;4r\x1b[4;1H\n\n\n"),
      ("reset region", b"\x1b[r"),
      ("alt screen", b"\x1b[?1049h\x1b[Halt text"),
      ("main screen", b"\x1b[?1049l"),
      ("flood", flood.as_bytes()),
    ];
    for (step, bytes) in steps {
      terminal.vt_write(bytes);
      check(&terminal, step);
    }
    terminal.scroll_viewport(libghostty_vt::terminal::ScrollViewport::Delta(-7));
    check(&terminal, "viewport scroll");
    terminal.scroll_viewport(libghostty_vt::terminal::ScrollViewport::Bottom);
    check(&terminal, "scroll to bottom");
    terminal.resize(8, 6, 8, 16).unwrap();
    check(&terminal, "resize");
  }

  fn search_handle(cols: u16, rows: u16) -> PaneHandle {
    PaneHandle::spawn(cols, rows, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap()
  }

  fn wait_search(
    handle: &PaneHandle,
    pred: impl Fn(&crate::terminal::snapshot::SearchSnapshot) -> bool,
  ) -> Arc<PaneSnapshot> {
    wait_snapshot(handle, |snap| snap.search.as_ref().is_some_and(&pred))
  }

  const SCROLLED: &[u8] = b"hello one\r\nfiller\r\nhello two\r\nfiller\r\nHELLO three\r\nfiller";

  #[test]
  fn search_counts_matches_across_scrollback_and_selects_the_newest() {
    let handle = search_handle(20, 3);
    handle.push_bytes(SCROLLED);
    wait_snapshot(&handle, |snap| snap.row_text(2) == "filler");
    assert!(handle.snapshot().unwrap().search.is_none(), "no search until asked");
    handle.send(PaneCommand::SearchQuery("hello".into()));
    let snap = wait_search(&handle, |search| search.total == 3);
    let search = snap.search.as_ref().unwrap();
    assert_eq!(search.current, Some(3), "the newest match is selected first");
    assert_eq!(snap.viewport_offset, 0, "the newest match is already on screen");
    assert_eq!(search.matches.len(), 1, "one match fits in the 3 row viewport");
    let current = search.current_match[0];
    assert_eq!((current.y, current.x0, current.x1), (1, 0, 4));
    assert_eq!(snap.row_text(current.y), "HELLO three");
  }

  #[test]
  fn search_steps_wrap_and_scroll_the_viewport_to_the_match() {
    let handle = search_handle(20, 3);
    handle.push_bytes(SCROLLED);
    handle.send(PaneCommand::SearchQuery("hello".into()));
    wait_search(&handle, |search| search.total == 3 && search.current == Some(3));
    // Enter wraps from the newest match to the oldest and scrolls up to it.
    handle.send(PaneCommand::SearchStep { forward: true });
    let snap = wait_search(&handle, |search| search.current == Some(1));
    assert!(snap.viewport_offset > 0, "the oldest match is in scrollback");
    let span = snap.search.as_ref().unwrap().current_match[0];
    assert_eq!(snap.row_text(span.y), "hello one");
    handle.send(PaneCommand::SearchStep { forward: true });
    let snap = wait_search(&handle, |search| search.current == Some(2));
    let span = snap.search.as_ref().unwrap().current_match[0];
    assert_eq!(snap.row_text(span.y), "hello two");
    handle.send(PaneCommand::SearchStep { forward: false });
    wait_search(&handle, |search| search.current == Some(1));
    // Shift+Enter from the oldest wraps to the newest.
    handle.send(PaneCommand::SearchStep { forward: false });
    let snap = wait_search(&handle, |search| search.current == Some(3));
    assert_eq!(snap.viewport_offset, 0);
  }

  #[test]
  fn search_reports_no_results_follows_new_output_and_clears() {
    let handle = search_handle(20, 3);
    handle.push_bytes(SCROLLED);
    handle.send(PaneCommand::SearchQuery("zzz".into()));
    let snap = wait_search(&handle, |search| search.total == 0);
    let search = snap.search.as_ref().unwrap();
    assert_eq!(search.current, None);
    assert!(search.matches.is_empty() && search.current_match.is_empty());
    handle.send(PaneCommand::SearchQuery("hello".into()));
    wait_search(&handle, |search| search.total == 3);
    handle.push_bytes(b"\r\nhello four");
    wait_search(&handle, |search| search.total == 4);
    handle.send(PaneCommand::SearchQuery(String::new()));
    wait_snapshot(&handle, |snap| snap.search.is_none() && snap.seq > 1);
    handle.push_bytes(b"\r\nhello five");
    let snap = wait_snapshot(&handle, |snap| snap.row_text(2) == "hello five");
    assert!(snap.search.is_none(), "a closed search stays closed");
  }

  #[test]
  fn a_match_that_wraps_highlights_every_row_it_covers() {
    let handle = search_handle(10, 4);
    handle.push_bytes(b"abcdefghijklmnop");
    handle.send(PaneCommand::SearchQuery("hijkl".into()));
    let snap = wait_search(&handle, |search| search.total == 1);
    let spans: Vec<_> = snap
      .search
      .as_ref()
      .unwrap()
      .current_match
      .iter()
      .map(|span| (span.y, span.x0, span.x1))
      .collect();
    assert_eq!(spans, [(0, 7, 9), (1, 0, 1)]);
  }

  const READ_CLIPBOARD: &[u8] = b"\x1b]52;c;?\x1b\\";
  const EMPTY_REPLY: &[u8] = b"\x1b]52;c;\x1b\\";

  fn clipboard_reply(handler: ClipboardReadHandler) -> Vec<u8> {
    let (writer, collected) = collecting_writer();
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), handler).unwrap();
    handle.push_bytes(READ_CLIPBOARD);
    wait_bytes(&collected, |bytes| bytes.ends_with(b"\x1b\\"))
  }

  #[test]
  fn clipboard_read_answers_with_the_allowed_text() {
    let reply = clipboard_reply(Box::new(|request| request.allow("secret".into())));
    assert_eq!(reply, b"\x1b]52;c;c2VjcmV0\x1b\\");
  }

  #[test]
  fn clipboard_read_answers_empty_when_denied_or_unanswered() {
    assert_eq!(clipboard_reply(Box::new(|request| request.deny())), EMPTY_REPLY);
    assert_eq!(
      clipboard_reply(Box::new(drop)),
      EMPTY_REPLY,
      "a dropped request is a denial"
    );
  }

  #[test]
  fn clipboard_read_waits_for_a_late_answer() {
    let reply = clipboard_reply(Box::new(|request| {
      thread::spawn(move || {
        thread::sleep(Duration::from_millis(150));
        request.allow("later".into());
      });
    }));
    assert_eq!(reply, b"\x1b]52;c;bGF0ZXI=\x1b\\");
  }

  #[test]
  fn closing_the_pane_ends_a_clipboard_read_that_nobody_answers() {
    let exited = Arc::new(AtomicBool::new(false));
    let flag = ExitFlag(Arc::clone(&exited));
    let writer: PtyWriter = Box::new(move |_| {
      let _ = &flag;
    });
    let held = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&held);
    let handler: ClipboardReadHandler = Box::new(move |request| sink.lock().unwrap().push(request));
    let handle = PaneHandle::spawn(20, 4, None, writer, Box::new(|| {}), handler).unwrap();
    handle.push_bytes(READ_CLIPBOARD);
    let deadline = Instant::now() + Duration::from_secs(2);
    while held.lock().unwrap().is_empty() && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(held.lock().unwrap().len(), 1, "the request must reach the handler");
    drop(handle);
    let deadline = Instant::now() + Duration::from_secs(2);
    while !exited.load(Ordering::Acquire) && Instant::now() < deadline {
      thread::sleep(Duration::from_millis(5));
    }
    assert!(
      exited.load(Ordering::Acquire),
      "the pane thread kept waiting for an answer"
    );
  }

  const RED: [u8; 4] = [255, 0, 0, 255];
  const BLUE: [u8; 4] = [0, 0, 255, 255];

  /// A Kitty graphics command carrying `payload` as base64.
  fn kitty(control: &str, payload: &[u8]) -> Vec<u8> {
    use base64::Engine;
    let encoded = base64::engine::general_purpose::STANDARD.encode(payload);
    format!("\x1b_G{control};{encoded}\x1b\\").into_bytes()
  }

  fn graphics_handle() -> PaneHandle {
    PaneHandle::spawn(20, 6, None, noop_writer(), Box::new(|| {}), deny_clipboard()).unwrap()
  }

  /// Waits until the pane has processed everything written so far.
  fn flush(handle: &PaneHandle, marker: &str) -> Arc<PaneSnapshot> {
    handle.push_bytes(format!("\x1b[6;1H{marker}").as_bytes());
    wait_snapshot(handle, |snap| snap.row_text(5) == marker)
  }

  #[test]
  fn a_direct_rgba_transmit_and_place_shows_one_image() {
    let handle = graphics_handle();
    let pixels: Vec<u8> = [RED, [0, 255, 0, 255], BLUE, [255, 255, 255, 255]].concat();
    handle.push_bytes(&kitty("a=T,f=32,s=2,v=2,q=2", &pixels));
    let snap = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    assert_eq!(snap.images.len(), 1);
    let image = &snap.images[0];
    assert_eq!((image.width, image.height), (2, 2), "drawn at its own size");
    assert_eq!((image.grid_cols, image.grid_rows), (1, 1));
    assert_eq!((image.col, image.row), (0, 0));
    assert_eq!((image.source_width, image.source_height), (2, 2));
    assert_eq!(image.layer, ImageLayer::AboveText, "z defaults to 0, above the text");
    assert_eq!((image.pixels.width, image.pixels.height), (2, 2));
    assert_eq!(image.pixels.rgba, pixels);
    assert_eq!((snap.cell_width, snap.cell_height), (8, 16));
  }

  #[test]
  fn an_rgb_transmit_is_widened_to_rgba() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=24,s=2,v=1,q=2", &[1, 2, 3, 4, 5, 6]));
    let snap = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    assert_eq!(snap.images[0].pixels.rgba, [1, 2, 3, 255, 4, 5, 6, 255]);
  }

  #[test]
  fn z_index_picks_the_layer_and_orders_the_list() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=1,z=5,q=2", &RED));
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=2,z=-1,q=2", &BLUE));
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=3,z=-1073741825,q=2", &RED));
    let snap = wait_snapshot(&handle, |snap| snap.images.len() == 3);
    let order: Vec<(u32, ImageLayer, i32)> = snap
      .images
      .iter()
      .map(|image| (image.pixels.id, image.layer, image.z))
      .collect();
    assert_eq!(
      order,
      [
        (3, ImageLayer::BelowBackground, -1_073_741_825),
        (2, ImageLayer::BelowText, -1),
        (1, ImageLayer::AboveText, 5),
      ]
    );
  }

  #[test]
  fn a_delete_command_removes_the_image() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=7,q=2", &RED));
    wait_snapshot(&handle, |snap| snap.images.len() == 1);
    handle.push_bytes(b"\x1b_Ga=d,d=I,i=7,q=2\x1b\\");
    let snap = wait_snapshot(&handle, |snap| snap.images.is_empty());
    assert!(snap.images.is_empty());
  }

  #[test]
  fn a_png_transmit_is_decoded() {
    use base64::Engine;
    // A 1x1 opaque red PNG.
    let png = base64::engine::general_purpose::STANDARD
      .decode("iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR4nGP4z8DwHwAFAAH/iZk9HQAAAABJRU5ErkJggg==")
      .unwrap();
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=100,q=2", &png));
    let snap = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    let pixels = &snap.images[0].pixels;
    assert_eq!((pixels.width, pixels.height), (1, 1));
    assert_eq!(pixels.rgba, RED);
  }

  #[test]
  fn a_garbage_png_is_refused() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=100,q=2", b"this is not a png"));
    assert!(flush(&handle, "done").images.is_empty());
  }

  #[test]
  fn a_clean_frame_shares_the_image_list_and_its_pixels() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,q=2", &RED));
    let first = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    handle.push_bytes(b"\x1b[3;1Hplain text");
    let second = wait_snapshot(&handle, |snap| snap.seq > first.seq && snap.row_text(2) == "plain text");
    assert!(Arc::ptr_eq(&first.images, &second.images), "the list was rebuilt");
    assert!(
      Arc::ptr_eq(&first.images[0].pixels, &second.images[0].pixels),
      "the pixels were copied"
    );
    // A second image changes the list but not the pixels of the image already in it.
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=9,q=2", &BLUE));
    let third = wait_snapshot(&handle, |snap| snap.images.len() == 2);
    let kept = third.images.iter().find(|image| image.pixels.id != 9).unwrap();
    assert!(
      Arc::ptr_eq(&first.images[0].pixels, &kept.pixels),
      "an unchanged image was copied again"
    );
  }

  #[test]
  fn a_retransmitted_image_gets_new_pixels() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=4,q=2", &RED));
    let first = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,i=4,q=2", &BLUE));
    let second = wait_snapshot(&handle, |snap| {
      snap.images.first().is_some_and(|i| i.pixels.rgba == BLUE)
    });
    assert_eq!(second.images.len(), 1);
    assert_ne!(first.images[0].pixels.generation, second.images[0].pixels.generation);
  }

  #[test]
  fn an_image_scrolls_with_the_text_and_leaves_with_it() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,s=1,v=1,q=2", &RED));
    let placed = wait_snapshot(&handle, |snap| !snap.images.is_empty());
    assert_eq!(placed.images[0].row, 0);
    handle.push_bytes(b"\x1b[6;1H\n\n");
    let moved = wait_snapshot(&handle, |snap| snap.images.first().is_none_or(|image| image.row < 0));
    assert!(moved.images.first().is_none_or(|image| image.row <= -2));
    handle.push_bytes(&"\n".repeat(30).into_bytes());
    let gone = wait_snapshot(&handle, |snap| snap.images.is_empty());
    assert!(gone.images.is_empty());
  }

  #[test]
  fn file_and_shared_memory_mediums_are_refused_without_reading() {
    let dir = tempfile::TempDir::new().unwrap();
    let file = dir.path().join("pixels.rgba");
    std::fs::write(&file, RED).unwrap();
    let temp = std::env::temp_dir().join(format!("tty-graphics-protocol-{}.rgba", std::process::id()));
    std::fs::write(&temp, RED).unwrap();
    let handle = graphics_handle();
    for (medium, path) in [("f", &file), ("t", &temp)] {
      handle.push_bytes(&kitty(
        &format!("a=T,t={medium},f=32,s=1,v=1,q=2"),
        path.to_str().unwrap().as_bytes(),
      ));
    }
    handle.push_bytes(&kitty("a=T,t=s,f=32,s=1,v=1,q=2", b"/deathpush-test-shm"));
    let snap = flush(&handle, "done");
    assert!(snap.images.is_empty(), "a refused medium showed an image");
    assert!(file.exists(), "the file medium touched the file");
    assert!(temp.exists(), "the temp-file medium read and deleted the file");
    std::fs::remove_file(&temp).unwrap();
  }

  #[test]
  fn the_pane_leaves_the_file_mediums_off_and_the_storage_limit_on() {
    let mut terminal = Terminal::new(10, 3).unwrap();
    super::configure_kitty_graphics(&mut terminal);
    assert_eq!(terminal.kitty_image_storage_limit().unwrap(), 320 * 1024 * 1024);
    assert!(!terminal.is_kitty_image_from_file_allowed().unwrap());
    assert!(!terminal.is_kitty_image_from_shared_mem_allowed().unwrap());
    assert_eq!(terminal.kitty_image_temp_file_dir().unwrap(), None);
  }

  /// Distinct image generations a pane publishes within `window`, polled fast enough to see every frame.
  fn generations_seen(handle: &PaneHandle, window: Duration, want: usize) -> Vec<u64> {
    let mut seen = Vec::new();
    let deadline = Instant::now() + window;
    while Instant::now() < deadline && seen.len() < want {
      if let Some(image) = handle.snapshot().and_then(|snap| snap.images.first().cloned()) {
        let generation = image.pixels.generation;
        if !seen.contains(&generation) {
          seen.push(generation);
        }
      }
      thread::sleep(Duration::from_millis(1));
    }
    seen
  }

  #[test]
  fn an_animation_plays_by_itself_and_stops_when_told() {
    let handle = graphics_handle();
    handle.push_bytes(&kitty("a=T,f=32,i=2,s=1,v=1,q=2", &RED));
    handle.push_bytes(&kitty("a=f,f=32,i=2,s=1,v=1,z=40,q=2", &BLUE));
    handle.push_bytes(b"\x1b_Ga=a,i=2,r=1,z=30,q=2\x1b\\");
    handle.push_bytes(b"\x1b_Ga=a,i=2,s=3,q=2\x1b\\");
    // No more bytes: every later frame comes from the pane's own timer.
    let seen = generations_seen(&handle, Duration::from_secs(2), 4);
    assert!(seen.len() >= 4, "the animation stalled after {} frames", seen.len());
    handle.push_bytes(b"\x1b_Ga=a,i=2,s=1,q=2\x1b\\");
    let stopped = flush(&handle, "stop");
    thread::sleep(Duration::from_millis(150));
    let settled = handle.snapshot().unwrap().seq;
    thread::sleep(Duration::from_millis(150));
    assert_eq!(
      handle.snapshot().unwrap().seq,
      settled,
      "a stopped animation kept waking the pane"
    );
    assert!(settled >= stopped.seq);
  }
}
