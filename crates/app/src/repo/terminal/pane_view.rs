//! One focused terminal pane: paints snapshots and routes input.
//!
//! Create a wake pair, pass the callback to [`PaneHandle::spawn`], then
//! [`PaneView::new`] with the receiver.

use std::ops::Range;
use std::sync::Arc;
use std::time::Duration;

use deathpush_core::terminal::pane::{KeyInput, KeyMods, MouseButton as TermMouse, PaneCommand, PaneHandle};
use deathpush_core::terminal::snapshot::{PaneSnapshot, Rgb};
use deathpush_core::theme::{Rgba, UiPalette};
use futures::StreamExt;
use futures::channel::mpsc::{TryRecvError, UnboundedReceiver, unbounded};
use gpui_kit::*;

use super::bell::bell_flashes;
use super::element::{
  PaintCache, TerminalElement, clamp_sel_anchor, clamp_selection, on_key_down, on_key_up, paint_from_app, saturate,
};
use crate::config::AppConfig;
use crate::theme::{ActivePalette, hsla};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaneEvent {
  Focused(u64),
}

impl EventEmitter<PaneEvent> for PaneView {}

#[cfg(test)]
pub(crate) struct BlockedWake {
  pub entered: std::sync::Arc<std::sync::atomic::AtomicBool>,
  release: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

#[cfg(test)]
impl BlockedWake {
  pub fn spawn_handle() -> (std::sync::Arc<PaneHandle>, Self) {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::thread;
    use std::time::Duration;

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
    let handle = Arc::new(PaneHandle::spawn(20, 4, None, Box::new(|_| {}), wake).unwrap());
    (handle, Self { entered, release })
  }

  pub fn wait_entered(&self) {
    use std::sync::atomic::Ordering;
    use std::thread;
    use std::time::{Duration, Instant};

    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
      if self.entered.load(Ordering::Acquire) {
        return;
      }
      thread::sleep(Duration::from_millis(1));
    }
    panic!("wake did not run");
  }
}

#[cfg(test)]
impl Drop for BlockedWake {
  fn drop(&mut self) {
    self.release.store(true, std::sync::atomic::Ordering::Release);
  }
}

struct SentPress {
  key: String,
  mods: KeyMods,
}

pub struct PaneView {
  pub id: u64,
  handle: Option<Arc<PaneHandle>>,
  snapshot: Option<Arc<PaneSnapshot>>,
  focus_handle: FocusHandle,
  selection: Option<((u16, u16), (u16, u16))>,
  selection_anchor: Option<(u16, u16)>,
  forwarded_button: Option<TermMouse>,
  cell: Option<(Pixels, Pixels)>,
  active: bool,
  visible: bool,
  grid: Option<(u16, u16, u32, u32)>,
  pending_resize: Option<(u16, u16, u32, u32)>,
  resize_queued: bool,
  blink_on: bool,
  blink_task: Option<Task<()>>,
  wake_task: Option<Task<()>>,
  blur_sub: Option<Subscription>,
  focus_sub: Option<Subscription>,
  copy_consumed_key: Option<String>,
  sent_presses: Vec<SentPress>,
  wheel_accum: f32,
  marked_text: Option<String>,
  marked_selection: Range<usize>,
  paint_cache: PaintCache,
  flashing: bool,
  flash_task: Option<Task<()>>,
  vt_colors: Option<VtColors>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct VtColors {
  foreground: Rgb,
  background: Rgb,
  cursor: Rgb,
  ansi: [Rgb; 16],
}

pub(crate) fn vt_set_colors(palette: &UiPalette) -> PaneCommand {
  let colors = vt_colors_from_palette(palette);
  PaneCommand::SetColors {
    foreground: colors.foreground,
    background: colors.background,
    cursor: colors.cursor,
    ansi: colors.ansi,
  }
}

fn vt_colors_from_palette(palette: &UiPalette) -> VtColors {
  VtColors {
    foreground: rgb_from_rgba(palette.terminal_foreground),
    background: rgb_from_rgba(palette.terminal_background),
    cursor: rgb_from_rgba(palette.terminal_cursor),
    ansi: palette.terminal_ansi.map(rgb_from_rgba),
  }
}

fn rgb_from_rgba(color: Rgba) -> Rgb {
  Rgb(color.r, color.g, color.b)
}

/// Producer half of the pane-thread wake. Pass the callback to [`PaneHandle::spawn`].
pub fn wake_pair() -> (Box<dyn Fn() + Send>, UnboundedReceiver<()>) {
  let (tx, rx) = unbounded();
  (
    Box::new(move || {
      let _ = tx.unbounded_send(());
    }),
    rx,
  )
}

fn subscribe_wake(rx: UnboundedReceiver<()>, cx: &mut Context<PaneView>) -> Task<()> {
  cx.spawn(async move |this, cx| {
    let mut rx = rx;
    while rx.next().await.is_some() {
      loop {
        match rx.try_recv() {
          Ok(()) => {}
          Err(TryRecvError::Closed) => return,
          Err(TryRecvError::Empty) => break,
        }
      }
      if this
        .update(cx, |this, cx| {
          this.pull_snapshot();
          this.maybe_flash(cx);
          cx.notify();
        })
        .is_err()
      {
        return;
      }
    }
  })
}

impl PaneView {
  /// Installs the wake subscription: on wake, pulls [`PaneHandle::snapshot`] and notifies.
  pub fn new(id: u64, handle: Arc<PaneHandle>, wake_rx: UnboundedReceiver<()>, cx: &mut Context<Self>) -> Self {
    let wake_task = subscribe_wake(wake_rx, cx);
    let mut this = Self::build(id, Some(handle), cx);
    this.wake_task = Some(wake_task);
    this
  }

  fn build(id: u64, handle: Option<Arc<PaneHandle>>, cx: &mut Context<Self>) -> Self {
    Self {
      id,
      handle,
      snapshot: None,
      focus_handle: cx.focus_handle(),
      selection: None,
      selection_anchor: None,
      forwarded_button: None,
      cell: None,
      active: true,
      visible: true,
      grid: None,
      pending_resize: None,
      resize_queued: false,
      blink_on: true,
      blink_task: None,
      wake_task: None,
      blur_sub: None,
      focus_sub: None,
      copy_consumed_key: None,
      sent_presses: Vec::new(),
      wheel_accum: 0.0,
      marked_text: None,
      marked_selection: 0..0,
      paint_cache: PaintCache::default(),
      flashing: false,
      flash_task: None,
      vt_colors: None,
    }
  }

  #[cfg(test)]
  pub(crate) fn new_unthreaded(id: u64, cx: &mut Context<Self>) -> Self {
    Self::build(id, None, cx)
  }

  #[cfg(test)]
  fn new_unthreaded_with_wake(id: u64, wake_rx: UnboundedReceiver<()>, cx: &mut Context<Self>) -> Self {
    let wake_task = subscribe_wake(wake_rx, cx);
    let mut this = Self::build(id, None, cx);
    this.wake_task = Some(wake_task);
    this
  }

  pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
    if self.active != active {
      self.active = active;
      if !active {
        self.stop_blink();
      }
      cx.notify();
    }
  }

  pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
    if self.visible != visible {
      self.visible = visible;
      if !visible {
        self.stop_blink();
      }
      cx.notify();
    }
  }

  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  pub fn copy_selection(&self, cx: &mut App) -> Option<String> {
    let snap = self.snapshot.as_ref()?;
    let (start, end) = self.selection?;
    let text = snap.selection_text(start, end);
    if text.is_empty() {
      return None;
    }
    cx.write_to_clipboard(ClipboardItem::new_string(text.clone()));
    Some(text)
  }

  pub(crate) fn send(&self, command: PaneCommand) {
    if let Some(handle) = self.handle.as_ref() {
      handle.send(command);
    }
  }

  pub(crate) fn sync_vt_colors(&mut self, cx: &App) {
    if self.handle.is_none() {
      return;
    }
    let Some(palette) = cx.try_global::<ActivePalette>() else {
      return;
    };
    let colors = vt_colors_from_palette(&palette.0);
    if self.vt_colors.as_ref() == Some(&colors) {
      return;
    }
    self.vt_colors = Some(colors);
    self.send(PaneCommand::SetColors {
      foreground: colors.foreground,
      background: colors.background,
      cursor: colors.cursor,
      ansi: colors.ansi,
    });
  }

  pub(crate) fn mouse_tracking(&self) -> bool {
    self.handle.as_ref().is_some_and(|handle| handle.mouse_tracking())
  }

  fn pull_snapshot(&mut self) {
    if let Some(handle) = self.handle.as_ref() {
      self.snapshot = handle.snapshot();
    }
  }

  fn maybe_flash(&mut self, cx: &mut Context<Self>) {
    let Some(snap) = self.snapshot.as_ref() else {
      return;
    };
    if !snap.bell {
      return;
    }
    if !bell_flashes(AppConfig::get(cx).settings.terminal.bell_style) {
      return;
    }
    self.start_flash(cx);
  }

  fn start_flash(&mut self, cx: &mut Context<Self>) {
    self.flashing = true;
    self.flash_task = Some(cx.spawn(async move |this, cx| {
      cx.background_executor().timer(Duration::from_millis(120)).await;
      let _ = this.update(cx, |this, cx| {
        this.flashing = false;
        this.flash_task.take();
        cx.notify();
      });
    }));
  }

  /// Full-pane flash overlay is painted while this is true.
  pub(crate) fn flashing(&self) -> bool {
    self.flashing
  }

  pub(crate) fn snapshot(&self) -> Option<Arc<PaneSnapshot>> {
    self.snapshot.clone()
  }

  pub(crate) fn selection_range(&self) -> Option<((u16, u16), (u16, u16))> {
    self.selection
  }

  pub(crate) fn active(&self) -> bool {
    self.active
  }

  pub(crate) fn blink_on(&self) -> bool {
    self.blink_on
  }

  pub(crate) fn focus_handle(&self) -> &FocusHandle {
    &self.focus_handle
  }

  pub(crate) fn dragging(&self) -> bool {
    self.selection_anchor.is_some()
  }

  pub(crate) fn mouse_captured(&self) -> bool {
    self.dragging() || self.forwarded_button.is_some()
  }

  pub(crate) fn set_forwarded_button(&mut self, button: Option<TermMouse>) {
    self.forwarded_button = button;
  }

  pub(crate) fn forwarded_button(&self) -> Option<TermMouse> {
    self.forwarded_button
  }

  pub(crate) fn set_cell(&mut self, cell: (Pixels, Pixels)) {
    self.cell = Some(cell);
  }

  pub(crate) fn needs_resize(&self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> bool {
    let next = (cols, rows, cell_w, cell_h);
    self.grid != Some(next) && self.pending_resize != Some(next)
  }

  pub(crate) fn queue_resize(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> bool {
    self.pending_resize = Some((cols, rows, cell_w, cell_h));
    if self.resize_queued {
      return false;
    }
    self.resize_queued = true;
    true
  }

  pub(crate) fn flush_resize(&mut self, cx: &mut Context<Self>) {
    self.resize_queued = false;
    let Some((cols, rows, cell_w, cell_h)) = self.pending_resize.take() else {
      return;
    };
    if self.grid == Some((cols, rows, cell_w, cell_h)) {
      return;
    }
    self.remember_grid(cols, rows, cell_w, cell_h);
    self.send(PaneCommand::Resize {
      cols,
      rows,
      cell_w,
      cell_h,
    });
    if let Some(core) = cx
      .try_global::<crate::window::WindowRegistry>()
      .and_then(|reg| reg.core.clone())
      && let Err(err) = core.terminal_resize(self.id, cols, rows)
    {
      tracing::warn!(id = self.id, cols, rows, %err, "terminal resize failed");
    }
  }

  pub(crate) fn remember_grid(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) {
    if let Some((old_cols, old_rows, _, _)) = self.grid
      && (cols < old_cols || rows < old_rows)
    {
      self.selection = clamp_selection(self.selection, cols, rows);
      self.selection_anchor = self.selection_anchor.map(|anchor| clamp_sel_anchor(anchor, cols, rows));
    }
    self.grid = Some((cols, rows, cell_w, cell_h));
  }

  pub(crate) fn begin_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    self.selection = None;
    self.selection_anchor = Some(cell);
    cx.notify();
  }

  pub(crate) fn extend_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    if let Some(start) = self.selection_anchor {
      self.selection = (start != cell).then_some((start, cell));
      cx.notify();
    }
  }

  pub(crate) fn end_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    self.extend_selection(cell, cx);
    self.selection_anchor = None;
    cx.notify();
  }

  pub(crate) fn note_copy_consumed(&mut self, key: String) {
    self.copy_consumed_key = Some(key);
  }

  pub(crate) fn take_copy_consumed(&mut self, key: &str) -> bool {
    if self.copy_consumed_key.as_deref() == Some(key) {
      self.copy_consumed_key = None;
      true
    } else {
      false
    }
  }

  pub(crate) fn note_sent_key(&mut self, key: String, mods: KeyMods) {
    if self.sent_presses.iter().any(|press| press.key == key) {
      return;
    }
    self.sent_presses.push(SentPress { key, mods });
  }

  pub(crate) fn take_sent_key(&mut self, key: &str) -> Option<KeyMods> {
    let index = self.sent_presses.iter().rposition(|press| press.key == key)?;
    Some(self.sent_presses.remove(index).mods)
  }

  pub(crate) fn wheel_accum(&mut self) -> &mut f32 {
    &mut self.wheel_accum
  }

  pub(crate) fn marked_text(&self) -> Option<&str> {
    self.marked_text.as_deref()
  }

  pub(crate) fn paint_cache(&mut self) -> &mut PaintCache {
    &mut self.paint_cache
  }

  pub(crate) fn select_word_at(&mut self, x: u16, y: u16, cx: &mut Context<Self>) {
    if let Some(range) = self.snapshot.as_ref().and_then(|snap| snap.word_at(x, y)) {
      self.selection = Some(range);
      cx.notify();
    }
  }

  fn on_focus_lost(&mut self) {
    let presses = std::mem::take(&mut self.sent_presses);
    for press in presses {
      self.send(PaneCommand::Key(KeyInput {
        key: press.key,
        text: None,
        mods: press.mods,
        press: false,
      }));
    }
    self.copy_consumed_key = None;
    self.stop_blink();
  }

  fn ensure_blink(&mut self, cx: &mut Context<Self>) {
    if self.blink_task.is_some() {
      return;
    }
    self.blink_task = Some(cx.spawn(async move |this, cx| {
      loop {
        cx.background_executor().timer(Duration::from_millis(500)).await;
        if this
          .update(cx, |this, cx| {
            this.blink_on = !this.blink_on;
            cx.notify();
          })
          .is_err()
        {
          break;
        }
      }
    }));
  }

  fn stop_blink(&mut self) {
    self.blink_task.take();
    self.blink_on = true;
  }

  fn sync_blink(&mut self, focused: bool, cursor_blink: bool, cx: &mut Context<Self>) {
    if self.active && self.visible && focused && cursor_blink {
      self.ensure_blink(cx);
    } else {
      self.stop_blink();
    }
  }

  #[cfg(test)]
  pub fn set_snapshot(&mut self, snapshot: Arc<PaneSnapshot>) {
    self.snapshot = Some(snapshot);
  }

  #[cfg(test)]
  fn has_wake_task(&self) -> bool {
    self.wake_task.is_some()
  }

  #[cfg(test)]
  fn has_blink_task(&self) -> bool {
    self.blink_task.is_some()
  }

  #[cfg(test)]
  fn last_grid(&self) -> Option<(u16, u16, u32, u32)> {
    self.grid
  }

  #[cfg(test)]
  fn sent_press_count(&self) -> usize {
    self.sent_presses.len()
  }
}

fn utf16_len(text: &str) -> usize {
  text.encode_utf16().count()
}

fn utf16_to_utf8(text: &str, utf16_offset: usize, round_up: bool) -> usize {
  let mut remaining = utf16_offset;
  for (byte_i, ch) in text.char_indices() {
    if remaining == 0 {
      return byte_i;
    }
    let width = ch.len_utf16();
    if remaining < width {
      return if round_up { byte_i + ch.len_utf8() } else { byte_i };
    }
    remaining -= width;
  }
  text.len()
}

fn clamp_utf16_range(text: &str, range: Range<usize>) -> Range<usize> {
  let len = utf16_len(text);
  let start = range.start.min(len);
  let end = range.end.min(len).max(start);
  start..end
}

fn utf16_range_to_bytes(text: &str, range: Range<usize>) -> Range<usize> {
  let range = clamp_utf16_range(text, range);
  let start = utf16_to_utf8(text, range.start, false);
  let end = utf16_to_utf8(text, range.end, true);
  start.min(end)..end.max(start)
}

fn utf8_to_utf16(text: &str, byte_offset: usize) -> usize {
  let byte_offset = byte_offset.min(text.len());
  match text.get(..byte_offset) {
    Some(prefix) => utf16_len(prefix),
    None => {
      let start = text
        .char_indices()
        .map(|(i, _)| i)
        .take_while(|i| *i <= byte_offset)
        .last()
        .unwrap_or(0);
      utf16_len(&text[..start])
    }
  }
}

#[cfg(test)]
fn utf16_adjusted_range(text: &str, range: Range<usize>) -> Range<usize> {
  let bytes = utf16_range_to_bytes(text, range);
  utf8_to_utf16(text, bytes.start)..utf8_to_utf16(text, bytes.end)
}

impl EntityInputHandler for PaneView {
  fn text_for_range(
    &mut self,
    range: Range<usize>,
    adjusted_range: &mut Option<Range<usize>>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<String> {
    let text = self.marked_text.as_deref().unwrap_or("");
    let bytes = utf16_range_to_bytes(text, range);
    *adjusted_range = Some(utf8_to_utf16(text, bytes.start)..utf8_to_utf16(text, bytes.end));
    Some(text.get(bytes).unwrap_or("").to_string())
  }

  fn selected_text_range(
    &mut self,
    _ignore_disabled_input: bool,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<UTF16Selection> {
    Some(UTF16Selection {
      range: self.marked_selection.clone(),
      reversed: false,
    })
  }

  fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<Range<usize>> {
    self.marked_text.as_ref().map(|text| 0..utf16_len(text))
  }

  fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    self.marked_text = None;
    self.marked_selection = 0..0;
    cx.notify();
  }

  fn paste(&mut self, item: ClipboardItem, _window: &mut Window, _cx: &mut Context<Self>) {
    if let Some(text) = item.text() {
      self.send(PaneCommand::Paste(text));
    }
  }

  fn replace_text_in_range(
    &mut self,
    _range: Option<Range<usize>>,
    text: &str,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.marked_text = None;
    self.marked_selection = 0..0;
    if !text.is_empty() {
      self.send(PaneCommand::Text(text.to_string()));
    }
    cx.notify();
  }

  fn replace_and_mark_text_in_range(
    &mut self,
    _range: Option<Range<usize>>,
    new_text: &str,
    new_selected_range: Option<Range<usize>>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    if new_text.is_empty() {
      self.marked_text = None;
      self.marked_selection = 0..0;
    } else {
      let len = utf16_len(new_text);
      self.marked_text = Some(new_text.to_string());
      self.marked_selection = new_selected_range
        .map(|range| clamp_utf16_range(new_text, range))
        .unwrap_or(len..len);
    }
    cx.notify();
  }

  fn bounds_for_range(
    &mut self,
    _range_utf16: Range<usize>,
    element_bounds: Bounds<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<Bounds<Pixels>> {
    let (cell_w, cell_h) = self.cell?;
    let cursor = self.snapshot.as_ref()?.cursor.as_ref()?;
    Some(Bounds::new(
      point(
        element_bounds.origin.x + cell_w * usize::from(cursor.x),
        element_bounds.origin.y + cell_h * usize::from(cursor.y),
      ),
      size(cell_w, cell_h),
    ))
  }

  fn character_index_for_point(
    &mut self,
    _point: Point<Pixels>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<usize> {
    Some(0)
  }

  fn text_length_utf16(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<usize> {
    Some(self.marked_text.as_deref().map(utf16_len).unwrap_or(0))
  }
}

impl Render for PaneView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.sync_vt_colors(cx);
    let settings = paint_from_app(cx);
    let focused = self.focus_handle.is_focused(window);
    self.sync_blink(focused, settings.cursor_blink, cx);
    if self.blur_sub.is_none() {
      self.blur_sub = Some(cx.on_blur(&self.focus_handle, window, |this, _, _cx| {
        this.on_focus_lost();
      }));
    }
    if self.focus_sub.is_none() {
      self.focus_sub = Some(cx.on_focus(&self.focus_handle, window, |this, _, cx| {
        cx.emit(PaneEvent::Focused(this.id));
      }));
    }
    let view = cx.entity();
    let focus = self.focus_handle.clone();
    div()
      .id(("terminal-pane", self.id))
      .size_full()
      .p(px(8.0))
      .bg(hsla(saturate(settings.background, settings.saturation)))
      .overflow_hidden()
      .track_focus(&self.focus_handle)
      .key_context("Terminal")
      .opacity(if self.active { 1.0 } else { 0.7 })
      .on_key_down({
        let view = view.clone();
        move |event, _, cx| on_key_down(&view, event, cx)
      })
      .on_key_up({
        let view = view.clone();
        move |event, _, cx| on_key_up(&view, event, cx)
      })
      .on_mouse_down(MouseButton::Left, {
        let focus = focus.clone();
        let view = view.clone();
        move |_, window, cx| {
          view.update(cx, |this, cx| cx.emit(PaneEvent::Focused(this.id)));
          focus.focus(window, cx);
        }
      })
      .child(TerminalElement { view, settings })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::terminal::pane::PtyWriter;
  use deathpush_core::terminal::snapshot::{PaneSnapshot, Rgb, SnapshotCell};
  use gpui_kit::TestAppContext;

  use crate::config::AppConfig;
  use crate::repo::terminal::element::clamp_sel_anchor;

  struct PaneHost {
    pane: Entity<PaneView>,
  }

  impl Render for PaneHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      self.pane.clone()
    }
  }

  struct RepoPaneHost {
    pane: Entity<PaneView>,
    clear_fired: std::rc::Rc<std::cell::Cell<bool>>,
  }

  impl Render for RepoPaneHost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
      let fired = std::rc::Rc::clone(&self.clear_fired);
      div()
        .key_context(crate::keymap::CONTEXT_REPOSITORY)
        .size_full()
        .on_action(move |_: &crate::actions::ClearSelection, _, _| {
          fired.set(true);
        })
        .child(self.pane.clone())
    }
  }

  fn text_cell(ch: char) -> SnapshotCell {
    SnapshotCell {
      text: ch.to_string(),
      ..SnapshotCell::default()
    }
  }

  fn injected_snapshot(text: &str) -> Arc<PaneSnapshot> {
    let cols = text.chars().count() as u16;
    Arc::new(PaneSnapshot {
      seq: 1,
      cols,
      rows: 1,
      cells: text.chars().map(text_cell).collect(),
      cursor: None,
      background: Rgb(0, 0, 0),
      foreground: Rgb(255, 255, 255),
      cursor_color: None,
      viewport_offset: 0,
      scrollback_rows: 0,
      bell: false,
    })
  }

  #[test]
  fn selection_clamps_on_both_axes() {
    assert_eq!(clamp_sel_anchor((10, 5), 4, 3), (3, 2));
    assert_eq!(clamp_sel_anchor((1, 8), 4, 3), (1, 2));
    assert_eq!(clamp_sel_anchor((1, 1), 4, 3), (1, 1));
    let clamped = clamp_selection(Some(((9, 9), (8, 0))), 5, 2);
    assert_eq!(clamped, Some(((4, 1), (4, 0))));
  }

  #[gpui_kit::test]
  fn click_does_not_select_a_cell_but_drag_selects_text(cx: &mut TestAppContext) {
    let pane = cx.new(|cx| PaneView::new_unthreaded(1, cx));
    pane.update(cx, |view, cx| {
      view.begin_selection((4, 3), cx);
      assert!(view.selection.is_none());
      assert!(view.mouse_captured());
      view.end_selection((4, 3), cx);
      assert!(view.selection.is_none());
      assert!(!view.mouse_captured());

      view.begin_selection((2, 0), cx);
      view.extend_selection((5, 0), cx);
      assert_eq!(view.selection, Some(((2, 0), (5, 0))));
      view.end_selection((5, 0), cx);
      assert_eq!(view.selection, Some(((2, 0), (5, 0))));
      assert!(!view.mouse_captured());

      view.begin_selection((7, 4), cx);
      view.end_selection((7, 4), cx);
      assert!(view.selection.is_none(), "a click clears an existing selection");
    });
  }

  #[gpui_kit::test]
  fn second_paint_of_unchanged_snapshot_does_not_reshape(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      AppConfig::update(cx, |config| {
        config.settings.terminal.letter_spacing = 1.0;
        config.settings.terminal.cursor_blink = false;
      });
    });
    let snapshot = injected_snapshot("ab");
    let window = cx.add_window(move |_, cx| PaneHost {
      pane: cx.new(|cx| PaneView::new_unthreaded(1, cx)),
    });
    window
      .update(cx, |host, window, cx| {
        host.pane.update(cx, |view, cx| {
          view.set_snapshot(snapshot.clone());
          view.set_active(true, cx);
          view.focus(window, cx);
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    let first = window
      .update(cx, |host, _, cx| {
        host.pane.update(cx, |view, _| view.paint_cache().shape_calls())
      })
      .unwrap();
    assert!(first > 0, "spaced paint must shape cells");
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    let second = window
      .update(cx, |host, _, cx| {
        host.pane.update(cx, |view, _| view.paint_cache().shape_calls())
      })
      .unwrap();
    assert_eq!(second, first, "unchanged snapshot must not reshape");
  }

  #[gpui_kit::test]
  fn pane_view_renders_a_snapshot(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let snapshot = injected_snapshot("hi");
    let window = cx.add_window(move |_, cx| PaneHost {
      pane: cx.new(|cx| PaneView::new_unthreaded(1, cx)),
    });
    window
      .update(cx, |host, window, cx| {
        host.pane.update(cx, |view, cx| {
          view.set_snapshot(snapshot.clone());
          view.set_active(true, cx);
          view.focus(window, cx);
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |host, _, cx| {
        host.pane.update(cx, |view, _| {
          assert_eq!(view.snapshot.as_ref().map(|snap| snap.seq), Some(1));
          assert_eq!(view.snapshot.as_ref().unwrap().row_text(0), "hi");
        });
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn wake_task_cancels_when_the_view_drops(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let (tx, rx) = unbounded();
    let view = cx.new(|cx| PaneView::new_unthreaded_with_wake(1, rx, cx));
    view.update(cx, |view, _| {
      assert!(view.has_wake_task());
    });
    drop(view);
    let _ = tx.unbounded_send(());
    cx.run_until_parked();
  }

  #[gpui_kit::test]
  fn resize_coalesces_to_the_latest_grid(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let view = cx.new(|cx| PaneView::new_unthreaded(1, cx));
    view.update(cx, |view, cx| {
      view.selection = Some(((10, 5), (9, 8)));
      assert!(view.needs_resize(10, 4, 8, 16));
      assert!(view.queue_resize(10, 4, 8, 16));
      assert!(!view.needs_resize(10, 4, 8, 16));
      assert!(view.needs_resize(20, 8, 8, 16));
      assert!(!view.queue_resize(20, 8, 8, 16));
      assert!(!view.needs_resize(20, 8, 8, 16));
      view.flush_resize(cx);
      assert_eq!(view.last_grid(), Some((20, 8, 8, 16)));
      assert!(!view.needs_resize(20, 8, 8, 16));
      view.remember_grid(4, 3, 8, 16);
      assert_eq!(view.selection_range(), Some(((3, 2), (3, 2))));
    });
  }

  #[gpui_kit::test]
  fn blink_stops_when_inactive(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let window = cx.add_window(move |_, cx| PaneHost {
      pane: cx.new(|cx| PaneView::new_unthreaded(1, cx)),
    });
    window
      .update(cx, |host, window, cx| {
        host.pane.update(cx, |view, cx| {
          view.set_active(true, cx);
          view.focus(window, cx);
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    window
      .update(cx, |host, _, cx| {
        host.pane.update(cx, |view, cx| {
          assert!(view.has_blink_task());
          view.set_active(false, cx);
          assert!(!view.has_blink_task());
        });
      })
      .unwrap();
  }

  fn wait_bytes(buf: &std::sync::Mutex<Vec<u8>>, pred: impl Fn(&[u8]) -> bool) -> Vec<u8> {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
      let guard = buf.lock().unwrap();
      if pred(&guard) {
        return guard.clone();
      }
      drop(guard);
      std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for pty bytes");
  }

  #[test]
  fn utf16_ranges_cover_accented_and_surrogate_pairs() {
    assert_eq!(utf16_len("é"), 1);
    assert_eq!(utf16_len("😀"), 2);
    assert_eq!(utf16_len("é😀"), 3);
    assert_eq!(utf16_range_to_bytes("é", 0..1), 0..2);
    assert_eq!(utf16_range_to_bytes("😀", 0..2), 0..4);
    assert_eq!(utf16_range_to_bytes("😀", 0..1), 0..4);
    assert_eq!(utf16_range_to_bytes("a😀b", 1..3), 1..5);
    assert_eq!("é".get(utf16_range_to_bytes("é", 0..1)).unwrap(), "é");
    assert_eq!("😀".get(utf16_range_to_bytes("😀", 0..2)).unwrap(), "😀");
    assert_eq!("😀".get(utf16_range_to_bytes("😀", 1..2)).unwrap(), "😀");
    assert_eq!(utf16_adjusted_range("😀", 1..2), 0..2);
  }

  #[gpui_kit::test]
  fn sent_press_repeat_keeps_one_and_release_uses_original_mods(cx: &mut TestAppContext) {
    cx.update(gpui_kit::init);
    let view = cx.new(|cx| PaneView::new_unthreaded(1, cx));
    view.update(cx, |view, _| {
      let shift = KeyMods {
        shift: true,
        ..KeyMods::default()
      };
      view.note_sent_key("up".into(), shift.clone());
      view.note_sent_key("up".into(), KeyMods::default());
      assert_eq!(view.sent_press_count(), 1);
      assert_eq!(view.take_sent_key("up"), Some(shift.clone()));
      assert_eq!(view.take_sent_key("up"), None);

      view.note_sent_key("up".into(), shift.clone());
      view.note_sent_key(
        "up".into(),
        KeyMods {
          ctrl: true,
          ..KeyMods::default()
        },
      );
      assert_eq!(view.sent_press_count(), 1);
      view.on_focus_lost();
      assert_eq!(view.sent_press_count(), 0);
    });
  }

  #[gpui_kit::test]
  fn input_handler_uses_utf16_units(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let window = cx.add_window(|_, cx| PaneView::new_unthreaded(1, cx));
    window
      .update(cx, |view, window, cx| {
        view.replace_and_mark_text_in_range(None, "é😀", Some(1..3), window, cx);
        assert_eq!(view.marked_text_range(window, cx), Some(0..3));
        assert_eq!(view.text_length_utf16(window, cx), Some(3));
        let sel = view.selected_text_range(false, window, cx).unwrap();
        assert_eq!(sel.range, 1..3);
        let mut adjusted = None;
        assert_eq!(
          view.text_for_range(0..1, &mut adjusted, window, cx).as_deref(),
          Some("é")
        );
        assert_eq!(adjusted, Some(0..1));
        let mut adjusted = None;
        assert_eq!(
          view.text_for_range(1..3, &mut adjusted, window, cx).as_deref(),
          Some("😀")
        );
        assert_eq!(adjusted, Some(1..3));
        view.replace_and_mark_text_in_range(None, "😀", None, window, cx);
        let mut adjusted = None;
        assert_eq!(
          view.text_for_range(1..2, &mut adjusted, window, cx).as_deref(),
          Some("😀")
        );
        assert_eq!(adjusted, Some(0..2));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn printable_key_reaches_text_command_once(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let collected = Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = Arc::new(PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap());
    let (_, rx) = unbounded();
    let window = cx.add_window({
      let handle = Arc::clone(&handle);
      move |_, cx| PaneHost {
        pane: cx.new(|cx| PaneView::new(1, handle, rx, cx)),
      }
    });
    window
      .update(cx, |host, window, cx| {
        host.pane.update(cx, |view, cx| {
          view.set_active(true, cx);
          view.focus(window, cx);
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    cx.dispatch_keystroke(window.into(), Keystroke::parse("a").unwrap());
    cx.run_until_parked();
    let got = wait_bytes(&collected, |bytes| bytes == b"a");
    assert_eq!(got, b"a");
    window
      .update(cx, |host, _, cx| {
        host.pane.update(cx, |view, _| {
          assert_eq!(view.sent_press_count(), 0);
        });
      })
      .unwrap();
    drop(handle);
  }

  #[gpui_kit::test]
  fn escape_reaches_the_focused_pane_key_path(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      cx.bind_keys(crate::keymap::bindings());
    });
    let collected = Arc::new(std::sync::Mutex::new(Vec::new()));
    let writer_buf = Arc::clone(&collected);
    let writer: PtyWriter = Box::new(move |bytes| writer_buf.lock().unwrap().extend(bytes));
    let handle = Arc::new(PaneHandle::spawn(20, 4, None, writer, Box::new(|| {})).unwrap());
    let (_, rx) = unbounded();
    let clear_fired = std::rc::Rc::new(std::cell::Cell::new(false));
    let window = cx.add_window({
      let handle = Arc::clone(&handle);
      let clear_fired = std::rc::Rc::clone(&clear_fired);
      move |_, cx| RepoPaneHost {
        pane: cx.new(|cx| PaneView::new(1, handle, rx, cx)),
        clear_fired,
      }
    });
    window
      .update(cx, |host, window, cx| {
        host.pane.update(cx, |view, cx| {
          view.set_active(true, cx);
          view.focus(window, cx);
        });
        window.refresh();
      })
      .unwrap();
    AnyWindowHandle::from(window)
      .update(cx, |_, window, cx| {
        let _ = window.draw(cx);
      })
      .unwrap();
    cx.dispatch_keystroke(window.into(), Keystroke::parse("escape").unwrap());
    cx.run_until_parked();
    let got = wait_bytes(&collected, |bytes| bytes.contains(&0x1b));
    assert!(got.contains(&0x1b), "expected ESC in {got:?}");
    assert!(
      !clear_fired.get(),
      "ClearSelection must not fire while Terminal is focused"
    );
    drop(handle);
  }

  fn wait_snapshot(handle: &PaneHandle, pred: impl Fn(&PaneSnapshot) -> bool) -> Arc<PaneSnapshot> {
    let deadline = std::time::Instant::now() + Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
      if let Some(snap) = handle.snapshot()
        && pred(&snap)
      {
        return snap;
      }
      std::thread::sleep(Duration::from_millis(10));
    }
    panic!("timed out waiting for snapshot");
  }

  fn palette_rgb(color: deathpush_core::theme::Rgba) -> Rgb {
    Rgb(color.r, color.g, color.b)
  }

  #[gpui_kit::test]
  fn spawn_pane_queues_active_palette_and_theme_change_resends(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let handle = Arc::new(PaneHandle::spawn(20, 4, None, Box::new(|_| {}), Box::new(|| {})).unwrap());
    let first_bg = cx.update(|cx| {
      super::super::model::TerminalModel::queue_theme_colors(&handle, cx);
      palette_rgb(cx.global::<ActivePalette>().0.terminal_background)
    });
    wait_snapshot(&handle, |snap| snap.background == first_bg);
    cx.update(|cx| {
      crate::theme::apply_theme("ayu-light", deathpush_core::theme::ThemeKind::Light, None, cx);
    });
    let second_bg = cx.update(|cx| palette_rgb(cx.global::<ActivePalette>().0.terminal_background));
    assert_ne!(
      first_bg, second_bg,
      "theme change must pick a different terminal background"
    );
    let (_, rx) = unbounded();
    let view = cx.new({
      let handle = Arc::clone(&handle);
      move |cx| PaneView::new(1, handle, rx, cx)
    });
    view.update(cx, |view, cx| view.sync_vt_colors(cx));
    wait_snapshot(&handle, |snap| snap.background == second_bg);
    drop(handle);
  }
}
