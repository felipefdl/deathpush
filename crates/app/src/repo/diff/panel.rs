use std::sync::Arc;

use deathpush_core::config::layout::MainView;
use deathpush_core::config::settings::{DiffLayout, HunkSeparators, LineDiffType};
use deathpush_core::diff_view::{DiffRows, RowOptions, build_rows};
use deathpush_core::session::types::Intent;
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;

use super::header;
use super::highlight::Highlighted;
use super::rows::{self, RowPaint, RowsMetrics};
use super::states::{self, DiffKind, classify};
use crate::config::AppConfig;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::ActivePalette;

#[derive(Clone, Debug)]
pub struct Selection;

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

pub struct DiffPanel {
  model: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  rows: Option<Arc<DiffRows>>,
  metrics: RowsMetrics,
  rows_key: RowsKey,
  highlighter: Option<Arc<Highlighted>>,
  old_image: Option<Arc<Image>>,
  new_image: Option<Arc<Image>>,
  scroll: UniformListScrollHandle,
  h_scroll: ScrollHandle,
  #[allow(dead_code)]
  selection: Option<Selection>,
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
      rows: None,
      metrics: RowsMetrics::default(),
      rows_key: RowsKey::default(),
      highlighter: None,
      old_image: None,
      new_image: None,
      scroll: UniformListScrollHandle::new(),
      h_scroll: ScrollHandle::new(),
      selection: None,
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
      },
    }
    let plan = (|| {
      let state = self.model.read(cx).state();
      let Some(payload) = state.diff.as_ref() else {
        return Plan::Clear;
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
        },
        _ => Plan::Apply {
          key,
          rows: None,
          metrics: RowsMetrics::default(),
          highlighter: hash_changed.then_some(None),
          images: hash_changed.then_some(CachedImages::default()),
          reset_scroll: hash_changed,
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
      }
      Plan::Apply {
        key,
        rows,
        metrics,
        highlighter,
        images,
        reset_scroll,
      } => {
        self.rows = rows;
        self.metrics = metrics;
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
}

impl Render for DiffPanel {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.sync_rows(cx);
    let palette = cx.global::<ActivePalette>().0;
    let (layout, show_line_numbers, show_background, indicators, line_diff, font_family, font_size, line_height) = {
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
      )
    };
    let (selected, load_ready, kind) = {
      let state = self.model.read(cx).state();
      (
        state.selected_file.clone(),
        state.diff_load_id == Some(state.selected_load_id),
        classify(state.diff.as_ref()),
      )
    };
    let weak = cx.weak_entity();
    let mut root = div()
      .size_full()
      .flex()
      .flex_col()
      .track_focus(&self.focus_handle)
      .key_context("Diff");
    let Some(selection) = selected else {
      return root.child(states::render_empty(palette));
    };
    root = root.child(header::render_header(&selection, layout, weak.clone(), palette, cx));
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
          let list = uniform_list("diff-rows", count, move |range, _, _| {
            range
              .map(|index| rows::render_row(rows.as_ref(), index, &paint))
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
              .child(div().h_full().min_w(px(width)).child(list)),
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
      .update(cx, |panel, _, cx| {
        assert_eq!(panel.rows().map(|rows| rows.len()), Some(expected));
        let _ = cx;
      })
      .unwrap();
  }
}
