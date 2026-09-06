use std::cell::RefCell;
use std::collections::HashMap;
use std::mem;
use std::rc::Rc;
use std::sync::Arc;

use deathpush_core::config::layout::MainView;
use deathpush_core::config::settings::{DiffLayout, HunkSeparators, LineDiffType};
use deathpush_core::diff_view::{DiffRows, RowOptions, build_rows};
use deathpush_core::session::types::Intent;
use deathpush_core::types::{FileStatus, ResourceGroupKind};
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

use super::header;
use super::highlight::{Highlighted, Side};
use super::rows::{self, HunkOp, Layouts, RowInteract, RowPaint, RowsMetrics};
use super::selection::{Anchor, Selection, row_at};
use super::states::{self, DiffKind, classify};
use crate::actions::{ClearSelection, CopyDiffSelection};
use crate::config::AppConfig;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::ActivePalette;

#[derive(Clone, Default)]
struct CachedImages {
  old: Option<Arc<Image>>,
  new: Option<Arc<Image>>,
}

#[derive(Clone, PartialEq, Eq, Default)]
struct RowsKey {
  content_hash: String,
  layout: DiffLayout,
  line_diff: LineDiffType,
  separators: HunkSeparators,
}

/// How the diff panel paints: the SCM file, or a read-only commit file.
#[derive(Clone, PartialEq, Eq)]
pub enum DiffMode {
  Scm,
  /// Commit file. Empty `path` hides the body until a file is clicked.
  Commit {
    commit: String,
    path: String,
    status: FileStatus,
  },
}

pub struct DiffPanel {
  model: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  mode: DiffMode,
  rows: Option<Arc<DiffRows>>,
  metrics: RowsMetrics,
  rows_key: RowsKey,
  highlighter: Option<Arc<Highlighted>>,
  old_image: Option<Arc<Image>>,
  new_image: Option<Arc<Image>>,
  scroll: UniformListScrollHandle,
  h_scroll: ScrollHandle,
  selection: Option<Selection>,
  dragging: bool,
  layouts: Layouts,
  pending_layouts: Layouts,
  hunk_ids: Vec<String>,
  staged: bool,
  merge: bool,
  line_height: f32,
  focus_handle: FocusHandle,
}

impl DiffPanel {
  pub fn new(model: Entity<RepoModel>, layout: Entity<LayoutModel>, cx: &mut Context<Self>) -> Self {
    cx.observe(&model, |this, _, cx| {
      this.sync_rows(cx);
      cx.notify();
    })
    .detach();
    cx.observe_global::<AppConfig>(|this, cx| {
      this.sync_rows(cx);
      cx.notify();
    })
    .detach();
    Self {
      model,
      layout,
      mode: DiffMode::Scm,
      rows: None,
      metrics: RowsMetrics::default(),
      rows_key: RowsKey::default(),
      highlighter: None,
      old_image: None,
      new_image: None,
      scroll: UniformListScrollHandle::new(),
      h_scroll: ScrollHandle::new(),
      selection: None,
      dragging: false,
      layouts: Rc::new(RefCell::new(HashMap::new())),
      pending_layouts: Rc::new(RefCell::new(HashMap::new())),
      hunk_ids: Vec::new(),
      staged: false,
      merge: false,
      line_height: 20.0,
      focus_handle: cx.focus_handle(),
    }
  }

  #[allow(dead_code)]
  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  #[allow(dead_code)]
  pub fn rows(&self) -> Option<&Arc<DiffRows>> {
    self.rows.as_ref()
  }

  /// Switch between SCM and commit-file presentation.
  pub fn set_mode(&mut self, mode: DiffMode, cx: &mut Context<Self>) {
    if self.mode != mode {
      self.mode = mode;
      cx.notify();
    }
  }

  /// The panel's current presentation.
  pub fn mode(&self) -> &DiffMode {
    &self.mode
  }

  pub(crate) fn open_file_history(&self, path: String, window: &mut Window, cx: &mut Context<Self>) {
    self
      .layout
      .update(cx, |layout, cx| layout.select_main_view(MainView::History, cx));
    self.model.update(cx, |model, cx| {
      model.dispatch(Intent::OpenFileHistory { path }, window, cx)
    });
  }

  pub(crate) fn open_selected_in_editor(&self, cx: &mut Context<Self>) {
    let Some(path) = self
      .model
      .read(cx)
      .state()
      .selected_file
      .as_ref()
      .map(|file| file.path.clone())
    else {
      return;
    };
    self.model.update(cx, |model, cx| model.open_in_editor(path, cx));
  }

  #[cfg(test)]
  pub(crate) fn model(&self) -> &Entity<RepoModel> {
    &self.model
  }

  fn sync_rows(&mut self, cx: &App) {
    let (layout, line_diff, separators) = {
      let diff = &AppConfig::get(cx).settings.diff;
      (diff.layout, diff.line_diff_type, diff.hunk_separators)
    };
    enum Plan {
      Skip,
      Clear,
      Apply {
        key: RowsKey,
        rows: Option<Arc<DiffRows>>,
        metrics: RowsMetrics,
        highlighter: Option<Option<Arc<Highlighted>>>,
        images: Option<CachedImages>,
        reset_scroll: bool,
        hunk_ids: Vec<String>,
        staged: bool,
        merge: bool,
      },
    }
    let plan = (|| {
      let state = self.model.read(cx).state();
      let payload = match &self.mode {
        DiffMode::Scm => {
          if state.selected_file.is_none() {
            return Plan::Clear;
          }
          if !state.scm_diff_ready() {
            return Plan::Skip;
          }
          let Some(payload) = state.diff.as_ref() else {
            return Plan::Clear;
          };
          payload
        }
        DiffMode::Commit { commit, path, .. } => {
          if path.is_empty() {
            return Plan::Clear;
          }
          if !state.commit_diff_ready(commit, path) {
            return Plan::Skip;
          }
          let Some(payload) = state.commit_diff.as_ref() else {
            return Plan::Clear;
          };
          payload
        }
      };
      let key = RowsKey {
        content_hash: payload.content_hash.clone(),
        layout,
        line_diff,
        separators,
      };
      if self.rows_key == key {
        return Plan::Skip;
      }
      let hash_changed = self.rows_key.content_hash != key.content_hash;
      let hunk_ids: Vec<String> = payload.hunks.iter().map(|hunk| hunk.id.clone()).collect();
      let staged = payload.staged;
      let merge = matches!(self.mode, DiffMode::Scm)
        && state
          .selected_file
          .as_ref()
          .is_some_and(|file| file.group_kind == ResourceGroupKind::Merge);
      match classify(Some(payload)) {
        DiffKind::Text => {
          let rows = Arc::new(build_rows(
            payload,
            &RowOptions {
              layout: key.layout,
              line_diff: key.line_diff,
              separators: key.separators,
            },
          ));
          let metrics = RowsMetrics::from_rows(rows.as_ref());
          let rebuild_highlighter = self
            .highlighter
            .as_ref()
            .is_none_or(|highlighted| highlighted.hash != key.content_hash);
          Plan::Apply {
            key,
            rows: Some(rows),
            metrics,
            highlighter: rebuild_highlighter.then(|| Some(Arc::new(Highlighted::build(payload)))),
            images: hash_changed.then_some(CachedImages::default()),
            reset_scroll: hash_changed,
            hunk_ids,
            staged,
            merge,
          }
        }
        DiffKind::Image => Plan::Apply {
          key,
          rows: None,
          metrics: RowsMetrics::default(),
          highlighter: hash_changed.then_some(None),
          images: hash_changed.then(|| {
            let (old, new) = states::decode_images(payload);
            CachedImages { old, new }
          }),
          reset_scroll: hash_changed,
          hunk_ids,
          staged,
          merge,
        },
        _ => Plan::Apply {
          key,
          rows: None,
          metrics: RowsMetrics::default(),
          highlighter: hash_changed.then_some(None),
          images: hash_changed.then_some(CachedImages::default()),
          reset_scroll: hash_changed,
          hunk_ids,
          staged,
          merge,
        },
      }
    })();
    match plan {
      Plan::Skip => {}
      Plan::Clear => {
        self.rows = None;
        self.metrics = RowsMetrics::default();
        self.highlighter = None;
        self.old_image = None;
        self.new_image = None;
        self.rows_key = RowsKey::default();
        self.hunk_ids.clear();
        self.staged = false;
        self.merge = false;
        self.clear_text_selection();
        self.layouts.borrow_mut().clear();
        self.pending_layouts.borrow_mut().clear();
      }
      Plan::Apply {
        key,
        rows,
        metrics,
        highlighter,
        images,
        reset_scroll,
        hunk_ids,
        staged,
        merge,
      } => {
        self.rows = rows;
        self.metrics = metrics;
        self.hunk_ids = hunk_ids;
        self.staged = staged;
        self.merge = merge;
        self.clear_text_selection();
        self.layouts.borrow_mut().clear();
        self.pending_layouts.borrow_mut().clear();
        if let Some(highlighter) = highlighter {
          self.highlighter = highlighter;
        }
        if let Some(images) = images {
          self.old_image = images.old;
          self.new_image = images.new;
        }
        if reset_scroll {
          self.scroll.scroll_to_item(0, ScrollStrategy::Top);
          self.h_scroll.set_offset(point(px(0.0), px(0.0)));
        }
        self.rows_key = key;
      }
    }
  }

  fn has_text_selection(&self) -> bool {
    self.selection.as_ref().is_some_and(|sel| !sel.is_empty())
  }

  fn clear_text_selection(&mut self) {
    self.selection = None;
    self.dragging = false;
  }

  fn begin_selection(&mut self, anchor: Anchor, window: &mut Window, cx: &mut Context<Self>) {
    self.selection = Some(Selection { anchor, head: anchor });
    self.dragging = true;
    self.focus_handle.focus(window, cx);
    cx.notify();
  }

  fn update_head(&mut self, pos: Point<Pixels>, cx: &mut Context<Self>) {
    if !self.dragging {
      return;
    }
    let Some(rows) = self.rows.as_ref() else {
      return;
    };
    let Some(sel) = self.selection.as_mut() else {
      return;
    };
    let Some(head) = hit_anchor(
      &self.layouts,
      pos,
      sel.anchor.side,
      rows,
      self.line_height,
      &self.scroll,
      Some(sel.head.byte),
    ) else {
      return;
    };
    if sel.head != head {
      sel.head = head;
      cx.notify();
    }
  }

  fn copy_selection(&self, cx: &mut App) {
    let Some(sel) = self.selection.as_ref().filter(|sel| !sel.is_empty()) else {
      return;
    };
    let Some(rows) = self.rows.as_ref() else {
      return;
    };
    cx.write_to_clipboard(ClipboardItem::new_string(sel.text(rows)));
  }

  fn hunk_action(&self, op: HunkOp, hunk_id: String, window: &mut Window, cx: &mut Context<Self>) {
    let intent = match op {
      HunkOp::Stage => Intent::StageHunk { hunk_id },
      HunkOp::Unstage => Intent::UnstageHunk { hunk_id },
      HunkOp::Discard => Intent::DiscardHunk {
        hunk_id,
        confirmed: false,
      },
    };
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }
}

fn hit_anchor(
  layouts: &Layouts,
  pos: Point<Pixels>,
  side: Side,
  rows: &DiffRows,
  line_height: f32,
  scroll: &UniformListScrollHandle,
  fallback_byte: Option<usize>,
) -> Option<Anchor> {
  let n = rows.len();
  if n == 0 {
    return None;
  }
  let map = layouts.borrow();
  let mut min_top: Option<Pixels> = None;
  for ((row, layout_side), layout) in map.iter() {
    if *layout_side != side && matches!(rows, DiffRows::SideBySide(_)) {
      continue;
    }
    let bounds = rows::ready_bounds(layout);
    min_top = Some(min_top.map_or(bounds.top(), |top| top.min(bounds.top())));
    if pos.y >= bounds.top() && pos.y < bounds.bottom() {
      let len = row_at(rows, *row, side).map(|row| row.text.len()).unwrap_or(0);
      let byte = rows::byte_at(layout, pos, len).or(fallback_byte)?;
      return Some(clamp_anchor(Anchor { row: *row, side, byte }, rows));
    }
  }
  let row = row_from_scroll(pos, n, line_height, scroll).unwrap_or_else(|| {
    if min_top.is_some_and(|top| pos.y < top) {
      0
    } else {
      n - 1
    }
  });
  let len = row_at(rows, row, side).map(|row| row.text.len()).unwrap_or(0);
  let byte = if let Some(layout) = map.get(&(row, side)) {
    map_byte_in_row(layout, pos, len).or(fallback_byte)
  } else {
    fallback_byte
  }?;
  Some(clamp_anchor(Anchor { row, side, byte }, rows))
}

fn map_byte_in_row(layout: &TextLayout, pos: Point<Pixels>, text_len: usize) -> Option<usize> {
  let bounds = rows::ready_bounds(layout);
  let mut mapped = pos;
  let top = f32::from(bounds.top());
  let bottom = f32::from(bounds.bottom());
  if bottom > top {
    mapped.y = px(f32::from(pos.y).clamp(top, (bottom - 0.01).max(top)));
  }
  rows::byte_at(layout, mapped, text_len)
}

fn clamp_anchor(mut anchor: Anchor, rows: &DiffRows) -> Anchor {
  let n = rows.len();
  if n == 0 {
    return anchor;
  }
  anchor.row = anchor.row.min(n - 1);
  if let Some(row) = row_at(rows, anchor.row, anchor.side) {
    anchor.byte = anchor.byte.min(row.text.len());
  }
  anchor
}

fn row_from_scroll(pos: Point<Pixels>, n: usize, line_height: f32, scroll: &UniformListScrollHandle) -> Option<usize> {
  if line_height <= 0.0 || n == 0 {
    return None;
  }
  let state = scroll.0.borrow();
  let bounds = state.base_handle.bounds();
  if bounds.size.height == px(0.0) {
    return None;
  }
  let offset = state.base_handle.offset();
  let y = f32::from(pos.y - bounds.origin.y) - f32::from(offset.y);
  let row = (y / line_height).floor();
  Some(row.clamp(0.0, (n - 1) as f32) as usize)
}

impl Render for DiffPanel {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    {
      let mut pending = self.pending_layouts.borrow_mut();
      if !pending.is_empty() {
        *self.layouts.borrow_mut() = mem::take(&mut *pending);
      }
    }
    self.sync_rows(cx);
    let palette = cx.global::<ActivePalette>().0;
    let (
      layout,
      show_line_numbers,
      show_background,
      indicators,
      line_diff,
      font_family,
      font_size,
      line_height,
      mut show_hunk_actions,
    ) = {
      let settings = &AppConfig::get(cx).settings;
      (
        settings.diff.layout,
        settings.diff.show_line_numbers,
        settings.diff.show_background,
        settings.diff.diff_indicators,
        settings.diff.line_diff_type,
        settings.editor.font_family.clone(),
        settings.editor.font_size,
        settings.editor.line_height,
        settings.diff.show_inline_hunk_actions,
      )
    };
    let commit_mode = match &self.mode {
      DiffMode::Commit { commit, path, status } => Some((commit.clone(), path.clone(), status.clone())),
      DiffMode::Scm => None,
    };
    let (selected, scm_load_ready, kind, commit_ready) = {
      let state = self.model.read(cx).state();
      let commit_ready = match &commit_mode {
        Some((commit, path, _)) => state.commit_diff_ready(commit, path),
        None => false,
      };
      let scm_load_ready = state.scm_diff_ready();
      let payload = match &self.mode {
        DiffMode::Scm if scm_load_ready => state.diff.as_ref(),
        DiffMode::Commit { commit, path, .. } if !path.is_empty() && commit_ready => state.commit_diff.as_ref(),
        _ => None,
      };
      (
        state.selected_file.clone(),
        scm_load_ready,
        classify(payload),
        commit_ready,
      )
    };
    if commit_mode.is_some() {
      show_hunk_actions = false;
    }
    let weak = cx.weak_entity();
    let mut root = div()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus_handle)
      .key_context("Diff")
      .on_action(cx.listener(|this, _: &CopyDiffSelection, _, cx| this.copy_selection(cx)))
      .on_action(cx.listener(|this, _: &ClearSelection, _, cx| {
        if this.has_text_selection() {
          this.clear_text_selection();
          cx.stop_propagation();
          cx.notify();
        }
      }));
    let load_ready = if let Some((_commit, path, status)) = commit_mode {
      if path.is_empty() {
        return root.child(div().flex_1().min_h_0());
      }
      root = root.child(header::render_commit_header(
        &path,
        status,
        layout,
        weak.clone(),
        palette,
      ));
      commit_ready
    } else {
      let Some(selection) = selected else {
        return root.child(states::render_empty(palette));
      };
      root = root.child(header::render_header(&selection, layout, weak.clone(), palette, cx));
      scm_load_ready
    };
    if !load_ready {
      return root.child(div().flex_1().min_h_0());
    }
    match kind {
      DiffKind::Empty => root.child(div().flex_1().min_h_0()),
      DiffKind::Image => root.child(states::render_image(
        self.old_image.clone(),
        self.new_image.clone(),
        palette,
      )),
      DiffKind::Binary => root.child(states::render_binary(weak, palette)),
      DiffKind::Large => root.child(states::render_large(weak, palette)),
      DiffKind::Text => match self.rows.clone() {
        Some(rows) => {
          self.line_height = line_height as f32;
          let family = rows::editor_font_family(&font_family);
          let font_size = font_size as f32;
          let advance = rows::measure_advance(window, family.as_ref(), font_size);
          let paint = RowPaint {
            palette,
            show_line_numbers,
            show_background,
            indicators,
            line_diff,
            line_height: line_height as f32,
            font_family: family,
            font_size,
            number_width: rows::number_width(self.metrics.max_line_number, advance),
            indicator_width: rows::indicator_width(indicators, advance),
            highlighter: self.highlighter.clone(),
            theme: cx.theme().highlight_theme.clone(),
          };
          let width = rows::content_width(&self.metrics, &paint, layout, advance);
          let count = rows.len();
          let scroll = self.scroll.clone();
          let layouts = self.layouts.clone();
          let pending = self.pending_layouts.clone();
          let interact = RowInteract {
            selection: self.selection,
            layouts: layouts.clone(),
            pending: pending.clone(),
            hunk_ids: Rc::new(self.hunk_ids.clone()),
            staged: self.staged,
            merge: self.merge,
            show_hunk_actions,
            side_by_side: matches!(rows.as_ref(), DiffRows::SideBySide(_)),
            on_mouse_down: Rc::new({
              let view = weak.clone();
              move |anchor, window, cx| {
                let _ = view.update(cx, |this, cx| this.begin_selection(anchor, window, cx));
              }
            }),
            on_hunk: Rc::new({
              let view = weak;
              move |op, hunk_id, window, cx| {
                let _ = view.update(cx, |this, cx| this.hunk_action(op, hunk_id, window, cx));
              }
            }),
          };
          let list = uniform_list("diff-rows", count, move |range, _, _| {
            layouts.borrow_mut().retain(|&(index, _), _| range.contains(&index));
            pending.borrow_mut().retain(|&(index, _), _| range.contains(&index));
            range
              .map(|index| rows::render_row(rows.as_ref(), index, &paint, &interact))
              .collect()
          })
          .size_full()
          .track_scroll(&scroll);
          root.child(
            div()
              .id("diff-h-scroll")
              .flex_1()
              .min_h_0()
              .overflow_x_scroll()
              .track_scroll(&self.h_scroll)
              .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, _, _, cx| {
                  if !this.dragging {
                    this.clear_text_selection();
                    cx.notify();
                  }
                }),
              )
              .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                if this.dragging {
                  this.update_head(event.position, cx);
                }
              }))
              .on_mouse_up(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                  this.dragging = false;
                }),
              )
              .on_mouse_up_out(
                MouseButton::Left,
                cx.listener(|this, _, _, _| {
                  this.dragging = false;
                }),
              )
              .child(div().size_full().min_w(px(width)).child(list)),
          )
        }
        None => root.child(div().flex_1().min_h_0()),
      },
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::Core;
  use deathpush_core::session::types::{
    DiffHunkPayload, DiffPayload, DiffPresence, FileSelection, OperationActions, SessionActions, SessionRepo,
    SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{DiffLine, RepoOperationState, ResourceGroupKind, StatusPhase};
  use gpui_kit::TestAppContext;

  use crate::config::AppConfig;
  use crate::repo::layout_model::LayoutModel;

  fn snapshot(root: &str, file: Option<FileSelection>) -> SessionSnapshot {
    SessionSnapshot {
      session_generation: 1,
      session_revision: 1,
      status_generation: 1,
      status_revision: 1,
      repo: SessionRepo {
        root: root.into(),
        head_branch: Some("main".into()),
        head_commit: Some("abc".into()),
        ahead: 0,
        behind: 0,
        operation_state: RepoOperationState::None,
        phase: StatusPhase::Settled,
      },
      groups: vec![],
      selection: SessionSelection { file, commit: None },
      scm: SessionScm::default(),
      actions: SessionActions {
        can_commit: false,
        commit_label: "Commit".into(),
        commit_destructive: false,
        can_stage_all: false,
        can_unstage_all: false,
        can_discard_all: false,
        discard_all_destructive: false,
        sync: SyncAction {
          enabled: false,
          kind: SyncKind::Fetch,
          destructive: false,
        },
        operation: OperationActions {
          continue_op: false,
          abort: false,
          skip: false,
          abort_destructive: false,
        },
      },
      last_commit: None,
      branches: vec![],
      stashes: vec![],
      tags: vec![],
      commit_log: vec![],
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  fn text_payload(modified: &str) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      original: String::new(),
      modified: modified.to_string(),
      language: Some("rust".into()),
      file_type: "text".into(),
      hunks: vec![DiffHunkPayload {
        id: "h".into(),
        header: "@@ -1,2 +1,2 @@".into(),
        old_start: 1,
        old_lines: 1,
        new_start: 1,
        new_lines: 2,
        lines: vec![
          DiffLine {
            content: "fn main() {}".into(),
            line_type: "context".into(),
            old_line_number: Some(1),
            new_line_number: Some(1),
          },
          DiffLine {
            content: "let x = 1;".into(),
            line_type: "add".into(),
            old_line_number: None,
            new_line_number: Some(2),
          },
        ],
      }],
      presence: DiffPresence {
        old_exists: true,
        new_exists: true,
      },
      editable: true,
      enable_line_selection: true,
      staged: false,
      content_hash: "h".into(),
    }
  }

  #[gpui_kit::test]
  fn text_payload_builds_rows(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let layout_dir = config_dir.path().to_path_buf();
    let root = layout_dir.to_string_lossy().into_owned();
    let payload = text_payload("fn main() {}\nlet x = 1;\n");
    let expected = build_rows(
      &payload,
      &RowOptions {
        layout: DiffLayout::SideBySide,
        line_diff: LineDiffType::WordAlt,
        separators: HunkSeparators::Simple,
      },
    )
    .len();
    let window = cx.add_window({
      let core = core.clone();
      let snapshot = snapshot(
        &root,
        Some(FileSelection {
          path: "src/main.rs".into(),
          staged: false,
          group_kind: ResourceGroupKind::WorkingTree,
        }),
      );
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |_, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        DiffPanel::new(model, layout, cx)
      }
    });
    window
      .update(cx, |panel, window, cx| {
        let payload = payload.clone();
        panel.model().update(cx, |model, _| {
          let load_id = model.state().selected_load_id;
          model.state_mut().diff = Some(payload);
          model.state_mut().diff_load_id = Some(load_id);
        });
        window.refresh();
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |panel, window, cx| {
        assert_eq!(panel.rows().map(|rows| rows.len()), Some(expected));
        panel.begin_selection(
          Anchor {
            row: 0,
            side: Side::New,
            byte: 0,
          },
          window,
          cx,
        );
        assert_eq!(window.focused(cx).as_ref(), Some(&panel.focus_handle));
      })
      .unwrap();

    crate::test_core::park_and_shutdown(cx, &core);
  }
}
