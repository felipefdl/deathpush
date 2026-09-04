//! One focused terminal pane: paints snapshots and routes input.
//!
//! Create a wake pair, pass the callback to [`PaneHandle::spawn`], then
//! [`PaneView::new`] with the receiver.

use std::sync::Arc;
use std::time::Duration;

use deathpush_core::terminal::pane::{KeyInput, KeyMods, MouseButton as TermMouse, PaneCommand, PaneHandle};
use deathpush_core::terminal::snapshot::PaneSnapshot;
use futures::StreamExt;
use futures::channel::mpsc::{TryRecvError, UnboundedReceiver, unbounded};
use gpui_kit::*;

use super::element::{PaintCache, TerminalElement, clamp_selection, paint_from_app};

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
  dragging: bool,
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
  copy_consumed_key: Option<String>,
  sent_presses: Vec<SentPress>,
  wheel_accum: f32,
  marked_text: Option<String>,
  paint_cache: PaintCache,
}

/// Producer half of the pane-thread wake. Pass the callback to [`PaneHandle::spawn`].
#[allow(dead_code)]
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
  #[allow(dead_code)]
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
      dragging: false,
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
      copy_consumed_key: None,
      sent_presses: Vec::new(),
      wheel_accum: 0.0,
      marked_text: None,
      paint_cache: PaintCache::default(),
    }
  }

  #[cfg(test)]
  fn new_unthreaded(id: u64, cx: &mut Context<Self>) -> Self {
    Self::build(id, None, cx)
  }

  #[cfg(test)]
  fn new_unthreaded_with_wake(id: u64, wake_rx: UnboundedReceiver<()>, cx: &mut Context<Self>) -> Self {
    let wake_task = subscribe_wake(wake_rx, cx);
    let mut this = Self::build(id, None, cx);
    this.wake_task = Some(wake_task);
    this
  }

  #[allow(dead_code)]
  pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
    if self.active != active {
      self.active = active;
      if !active {
        self.stop_blink();
      }
      cx.notify();
    }
  }

  #[allow(dead_code)]
  pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Self>) {
    if self.visible != visible {
      self.visible = visible;
      if !visible {
        self.stop_blink();
      }
      cx.notify();
    }
  }

  #[allow(dead_code)]
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

  pub(crate) fn mouse_tracking(&self) -> bool {
    self.handle.as_ref().is_some_and(|handle| handle.mouse_tracking())
  }

  fn pull_snapshot(&mut self) {
    if let Some(handle) = self.handle.as_ref() {
      self.snapshot = handle.snapshot();
    }
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
    self.dragging
  }

  pub(crate) fn mouse_captured(&self) -> bool {
    self.dragging || self.forwarded_button.is_some()
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
    }
    self.grid = Some((cols, rows, cell_w, cell_h));
  }

  pub(crate) fn begin_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    self.selection = Some((cell, cell));
    self.dragging = true;
    cx.notify();
  }

  pub(crate) fn extend_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    if let Some((start, _)) = self.selection {
      self.selection = Some((start, cell));
      cx.notify();
    }
  }

  pub(crate) fn end_selection(&mut self, cell: (u16, u16), cx: &mut Context<Self>) {
    if let Some((start, _)) = self.selection {
      self.selection = Some((start, cell));
    }
    self.dragging = false;
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
    self.sent_presses.push(SentPress { key, mods });
  }

  pub(crate) fn take_sent_key(&mut self, key: &str) -> bool {
    if let Some(index) = self.sent_presses.iter().rposition(|press| press.key == key) {
      self.sent_presses.remove(index);
      true
    } else {
      false
    }
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
}

impl EntityInputHandler for PaneView {
  fn text_for_range(
    &mut self,
    range: std::ops::Range<usize>,
    adjusted_range: &mut Option<std::ops::Range<usize>>,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<String> {
    let text = self.marked_text.as_deref().unwrap_or("");
    let start = range.start.min(text.len());
    let end = range.end.min(text.len());
    *adjusted_range = Some(start..end);
    Some(text.get(start..end).unwrap_or("").to_string())
  }

  fn selected_text_range(
    &mut self,
    _ignore_disabled_input: bool,
    _window: &mut Window,
    _cx: &mut Context<Self>,
  ) -> Option<UTF16Selection> {
    let len = self.marked_text.as_ref().map(String::len).unwrap_or(0);
    Some(UTF16Selection {
      range: len..len,
      reversed: false,
    })
  }

  fn marked_text_range(&self, _window: &mut Window, _cx: &mut Context<Self>) -> Option<std::ops::Range<usize>> {
    self.marked_text.as_ref().map(|text| 0..text.len())
  }

  fn unmark_text(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
    self.marked_text = None;
    cx.notify();
  }

  fn paste(&mut self, item: ClipboardItem, _window: &mut Window, _cx: &mut Context<Self>) {
    if let Some(text) = item.text() {
      self.send(PaneCommand::Paste(text));
    }
  }

  fn replace_text_in_range(
    &mut self,
    _range: Option<std::ops::Range<usize>>,
    text: &str,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.marked_text = None;
    if !text.is_empty() {
      self.send(PaneCommand::Text(text.to_string()));
    }
    cx.notify();
  }

  fn replace_and_mark_text_in_range(
    &mut self,
    _range: Option<std::ops::Range<usize>>,
    new_text: &str,
    _new_selected_range: Option<std::ops::Range<usize>>,
    _window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.marked_text = if new_text.is_empty() {
      None
    } else {
      Some(new_text.to_string())
    };
    cx.notify();
  }

  fn bounds_for_range(
    &mut self,
    _range_utf16: std::ops::Range<usize>,
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
}

impl Render for PaneView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let settings = paint_from_app(cx);
    let focused = self.focus_handle.is_focused(window);
    self.sync_blink(focused, settings.cursor_blink, cx);
    if self.blur_sub.is_none() {
      self.blur_sub = Some(cx.on_blur(&self.focus_handle, window, |this, _, _cx| {
        this.on_focus_lost();
      }));
    }
    let view = cx.entity();
    let focus = self.focus_handle.clone();
    div()
      .id(("terminal-pane", self.id))
      .size_full()
      .track_focus(&self.focus_handle)
      .key_context("Terminal")
      .opacity(if self.active { 1.0 } else { 0.7 })
      .on_mouse_down(MouseButton::Left, {
        let focus = focus.clone();
        move |_, window, cx| {
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
}
