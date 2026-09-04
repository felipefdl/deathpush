//! One focused terminal pane: paints snapshots and routes input.
//!
//! Create a wake pair, pass the callback to [`PaneHandle::spawn`], then
//! [`PaneView::new`] with the receiver.

use std::collections::HashSet;
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use deathpush_core::terminal::pane::{PaneCommand, PaneHandle};
use deathpush_core::terminal::snapshot::PaneSnapshot;
use gpui_kit::*;

use super::element::{TerminalElement, paint_from_app};

pub struct PaneView {
  pub id: u64,
  handle: Option<Arc<PaneHandle>>,
  snapshot: Option<Arc<PaneSnapshot>>,
  focus_handle: FocusHandle,
  selection: Option<((u16, u16), (u16, u16))>,
  dragging: bool,
  #[allow(dead_code)]
  cell: Option<(Pixels, Pixels)>,
  active: bool,
  grid: Option<(u16, u16, u32, u32)>,
  blink_on: bool,
  blink_task: Option<Task<()>>,
  copy_consumed_key: Option<String>,
  sent_keys: HashSet<String>,
}

/// Producer half of the pane-thread wake. Pass the callback to [`PaneHandle::spawn`].
#[allow(dead_code)]
pub fn wake_pair() -> (Box<dyn Fn() + Send>, Receiver<()>) {
  let (tx, rx) = mpsc::channel();
  (
    Box::new(move || {
      let _ = tx.send(());
    }),
    rx,
  )
}

#[allow(dead_code)]
fn subscribe_wake(rx: Receiver<()>, cx: &mut Context<PaneView>) {
  cx.spawn(async move |this, cx| {
    loop {
      cx.background_executor().timer(Duration::from_millis(8)).await;
      let mut woke = false;
      loop {
        match rx.try_recv() {
          Ok(()) => woke = true,
          Err(TryRecvError::Empty) => break,
          Err(TryRecvError::Disconnected) => return,
        }
      }
      if !woke {
        continue;
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
  .detach();
}

impl PaneView {
  /// Installs the wake subscription: on wake, pulls [`PaneHandle::snapshot`] and notifies.
  #[allow(dead_code)]
  pub fn new(id: u64, handle: Arc<PaneHandle>, wake_rx: Receiver<()>, cx: &mut Context<Self>) -> Self {
    subscribe_wake(wake_rx, cx);
    Self::build(id, Some(handle), cx)
  }

  fn build(id: u64, handle: Option<Arc<PaneHandle>>, cx: &mut Context<Self>) -> Self {
    Self {
      id,
      handle,
      snapshot: None,
      focus_handle: cx.focus_handle(),
      selection: None,
      dragging: false,
      cell: None,
      active: true,
      grid: None,
      blink_on: true,
      blink_task: None,
      copy_consumed_key: None,
      sent_keys: HashSet::new(),
    }
  }

  #[cfg(test)]
  fn new_unthreaded(id: u64, cx: &mut Context<Self>) -> Self {
    Self::build(id, None, cx)
  }

  #[allow(dead_code)]
  pub fn set_active(&mut self, active: bool, cx: &mut Context<Self>) {
    if self.active != active {
      self.active = active;
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

  #[allow(dead_code)]
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

  pub(crate) fn set_cell(&mut self, cell: (Pixels, Pixels)) {
    self.cell = Some(cell);
  }

  pub(crate) fn needs_resize(&self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) -> bool {
    self.grid != Some((cols, rows, cell_w, cell_h))
  }

  pub(crate) fn remember_grid(&mut self, cols: u16, rows: u16, cell_w: u32, cell_h: u32) {
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

  pub(crate) fn note_sent_key(&mut self, key: String) {
    self.sent_keys.insert(key);
  }

  pub(crate) fn take_sent_key(&mut self, key: &str) -> bool {
    self.sent_keys.remove(key)
  }

  pub(crate) fn select_word_at(&mut self, x: u16, y: u16, cx: &mut Context<Self>) {
    if let Some(range) = self.snapshot.as_ref().and_then(|snap| snap.word_at(x, y)) {
      self.selection = Some(range);
      cx.notify();
    }
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

  #[cfg(test)]
  pub fn set_snapshot(&mut self, snapshot: Arc<PaneSnapshot>) {
    self.snapshot = Some(snapshot);
  }
}

impl Render for PaneView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let settings = paint_from_app(cx);
    let focused = self.focus_handle.is_focused(window);
    if focused && settings.cursor_blink {
      self.ensure_blink(cx);
    } else {
      self.stop_blink();
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

  #[gpui_kit::test]
  fn pane_view_renders_a_snapshot(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let snapshot = injected_snapshot("hi");
    let window = cx.add_window(move |_, cx| PaneView::new_unthreaded(1, cx));
    window
      .update(cx, |view, window, cx| {
        view.set_snapshot(snapshot.clone());
        view.set_active(true, cx);
        view.focus(window, cx);
        window.refresh();
        assert_eq!(view.snapshot.as_ref().map(|snap| snap.seq), Some(1));
        assert_eq!(view.snapshot.as_ref().unwrap().row_text(0), "hi");
      })
      .unwrap();
  }
}
