use std::collections::HashMap;
use std::rc::Rc;

use deathpush_core::config::layout::MainView;
use deathpush_core::config::settings::{DiffIndicators, DiffLayout, HunkSeparators, LineDiffType, WordWrap};
use deathpush_core::content_hash::sha256_utf8;
use deathpush_core::session::types::{DiffHunkPayload, DiffPayload, Intent};
use deathpush_core::types::{FileStatus, ResourceGroupKind};
use gpui_kit::base::TextSelection;
use gpui_kit::component::ActiveTheme;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::component::diff::{
  Diff, DiffAnnotation, DiffChangeIndicator, DiffEvent, DiffFile, DiffHunkSeparator, DiffInlineUnit, DiffLinePosition,
  DiffMode as ViewMode, DiffSide, DiffState,
};
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::*;

use super::grammar::grammar_name;
use super::header;
use super::split::{SplitDiff, SplitState};
use super::split_document::{Document, Hunk};
use super::states::{self, DiffKind, classify};
use crate::actions::{ClearSelection, NextDiffChange, PreviousDiffChange};
use crate::config::AppConfig;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::ActivePalette;

#[derive(Clone, Default)]
struct CachedImages {
  old: Option<ImageSource>,
  new: Option<ImageSource>,
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

/// What a hunk button does to the hunk it sits on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HunkOp {
  Stage,
  Unstage,
  Discard,
}

pub(crate) fn hunk_intent(op: HunkOp, hunk_id: String) -> Intent {
  match op {
    HunkOp::Stage => Intent::StageHunk { hunk_id },
    HunkOp::Unstage => Intent::UnstageHunk { hunk_id },
    HunkOp::Discard => Intent::DiscardHunk {
      hunk_id,
      confirmed: false,
    },
  }
}

/// Where a hunk's button row hangs: under the hunk's first old line when the hunk has one, so the row sits at
/// the left edge of the diff, else under its first new line (a hunk that only adds).
pub(crate) fn hunk_anchor(path: &str, hunk: &DiffHunkPayload) -> DiffLinePosition {
  if hunk.old_lines > 0 {
    DiffLinePosition::new(path.to_string(), DiffSide::Original, hunk.old_start)
  } else {
    DiffLinePosition::new(path.to_string(), DiffSide::Modified, hunk.new_start)
  }
}

/// Whether the diff is a whole-file add or delete: every hunk has an empty old side, or every hunk has an
/// empty new side, so the other side's column would only push the text off screen. Such a diff always
/// shows inline.
pub(crate) fn one_sided(payload: &DiffPayload) -> bool {
  let empty_old = |hunk: &DiffHunkPayload| hunk.old_start == 0 && hunk.old_lines == 0;
  let empty_new = |hunk: &DiffHunkPayload| hunk.new_start == 0 && hunk.new_lines == 0;
  payload.merge_text.is_none()
    && !payload.hunks.is_empty()
    && (payload.hunks.iter().all(empty_old) || payload.hunks.iter().all(empty_new))
}

/// One annotation per hunk, plus the hunk id behind each anchor.
pub(crate) fn hunk_annotations(
  path: &str,
  hunks: &[DiffHunkPayload],
) -> (Vec<DiffAnnotation>, HashMap<DiffLinePosition, DiffHunkPayload>) {
  let mut annotations = Vec::with_capacity(hunks.len());
  let mut targets = HashMap::with_capacity(hunks.len());
  for hunk in hunks {
    let anchor = hunk_anchor(path, hunk);
    annotations.push(DiffAnnotation::line(
      SharedString::from(format!("hunk-actions-{}", hunk.id)),
      anchor.clone(),
    ));
    targets.insert(anchor, hunk.clone());
  }
  (annotations, targets)
}

pub(crate) fn view_mode(layout: DiffLayout) -> ViewMode {
  match layout {
    DiffLayout::SideBySide => ViewMode::Split,
    DiffLayout::Inline => ViewMode::Unified,
  }
}

pub(crate) fn inline_unit(line_diff: LineDiffType) -> Option<DiffInlineUnit> {
  match line_diff {
    LineDiffType::Word => Some(DiffInlineUnit::Word),
    LineDiffType::Char => Some(DiffInlineUnit::Character),
    LineDiffType::None => None,
  }
}

fn change_indicator(indicators: DiffIndicators) -> DiffChangeIndicator {
  match indicators {
    DiffIndicators::Classic => DiffChangeIndicator::Signs,
    DiffIndicators::Bars => DiffChangeIndicator::Bars,
    DiffIndicators::None => DiffChangeIndicator::None,
  }
}

fn hunk_separator(separators: HunkSeparators) -> DiffHunkSeparator {
  match separators {
    HunkSeparators::Simple => DiffHunkSeparator::Simple,
    HunkSeparators::Metadata => DiffHunkSeparator::Metadata,
    HunkSeparators::LineInfo => DiffHunkSeparator::LineInfo,
  }
}

/// The payload's files with its language applied, or none when it does not parse. A merge file shows its
/// working text with the conflicts as choices; everything else shows the patch.
fn parse_files(payload: &DiffPayload) -> Vec<DiffFile> {
  let parsed = match &payload.merge_text {
    Some(text) => DiffFile::parse_conflicts(payload.path.clone(), text).map(|file| vec![file]),
    None => DiffFile::parse(&payload.patch),
  };
  let files = match parsed {
    Ok(files) => files,
    Err(err) => {
      tracing::warn!("diff for {} did not parse: {err}", payload.path);
      return Vec::new();
    }
  };
  match payload.language.as_deref().and_then(grammar_name) {
    Some(name) => files.into_iter().map(|file| file.with_language(name)).collect(),
    None => files,
  }
}

pub struct DiffPanel {
  model: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  mode: DiffMode,
  state: Entity<DiffState>,
  split: Entity<SplitState>,
  /// Hash of the payload currently installed in `state` and `images`.
  installed: Option<String>,
  images: CachedImages,
  /// Hash of the merge text the installed conflicts were parsed from; a resolution writes back only over it.
  merge_base: Option<String>,
  /// The installed diff is a whole-file add or delete, shown inline whatever the layout setting.
  one_sided: bool,
  /// Frames still owed to a freshly installed diff. The viewer measures its viewport while it paints, and a
  /// notification raised mid-paint does not schedule the frame that applies it.
  settle_frames: u8,
}

impl DiffPanel {
  pub fn new(
    model: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&model, |this, _, cx| {
      this.sync(cx);
      cx.notify();
    })
    .detach();
    cx.observe_global::<AppConfig>(|this, cx| {
      this.sync(cx);
      cx.notify();
    })
    .detach();
    let (layout_setting, line_diff) = {
      let diff = &AppConfig::get(cx).settings.diff;
      (diff.layout, diff.line_diff_type)
    };
    let state = cx.new(|cx| {
      DiffState::new(Vec::new(), cx)
        .with_mode(view_mode(layout_setting))
        // The patch carries its own three lines of context; a resolved merge file shows whole.
        .with_context_lines(None)
        .with_inline_unit(inline_unit(line_diff))
    });
    // Scrolling, selection, and paint-time measurements change the state; the viewer repaints from the owner.
    cx.observe(&state, |_, _, cx| cx.notify()).detach();
    let split = cx.new(|cx| SplitState::new(state.clone(), cx));
    cx.observe(&split, |_, _, cx| cx.notify()).detach();
    // The last conflict of a merge file resolved: the resolved text becomes the working file.
    cx.subscribe_in(&state, window, |this, state, event: &DiffEvent, window, cx| {
      let DiffEvent::ConflictResolved(path, _) = event else {
        return;
      };
      let Some(text) = state.read(cx).resolved_text(path) else {
        return;
      };
      let Some(expected_hash) = this.merge_base.clone() else {
        return;
      };
      let path = path.to_string();
      this.model.update(cx, |model, cx| {
        model.write_resolved_conflicts(path, expected_hash, text, window, cx)
      });
    })
    .detach();
    Self {
      model,
      layout,
      mode: DiffMode::Scm,
      state,
      split,
      installed: None,
      merge_base: None,
      one_sided: false,
      images: CachedImages::default(),
      settle_frames: 0,
    }
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

  /// Scroll to the next changed group after the viewport.
  pub(crate) fn next_change(&mut self, cx: &mut Context<Self>) {
    if self.uses_split(cx) {
      self.split.update(cx, |split, cx| split.move_change(true, cx));
    } else {
      self.state.update(cx, |state, cx| state.next_change(cx));
    }
  }

  /// Scroll to the previous changed group before the viewport.
  pub(crate) fn previous_change(&mut self, cx: &mut Context<Self>) {
    if self.uses_split(cx) {
      self.split.update(cx, |split, cx| split.move_change(false, cx));
    } else {
      self.state.update(cx, |state, cx| state.previous_change(cx));
    }
  }

  #[cfg(test)]
  pub(crate) fn model(&self) -> &Entity<RepoModel> {
    &self.model
  }

  #[cfg(test)]
  pub(crate) fn state(&self) -> &Entity<DiffState> {
    &self.state
  }

  /// Escape: clear the text and line selection. Returns whether there was one.
  fn clear_selection(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
    let had_lines = self.state.read(cx).selected_lines().is_some();
    if !had_lines && !TextSelection::has_selection(window, cx) {
      return false;
    }
    TextSelection::clear(window, cx);
    self.state.update(cx, |state, cx| state.set_selected_lines(None, cx));
    self.split.update(cx, |split, cx| split.clear_selection(cx));
    true
  }

  /// The payload this panel shows, when it is loaded for the current selection.
  fn ready_payload<'a>(&self, cx: &'a App) -> Option<&'a DiffPayload> {
    let state = self.model.read(cx).state();
    match &self.mode {
      DiffMode::Scm => {
        if state.selected_file.is_none() || !state.scm_diff_ready() {
          return None;
        }
        state.diff.as_ref()
      }
      DiffMode::Commit { commit, path, .. } => {
        if path.is_empty() || !state.commit_diff_ready(commit, path) {
          return None;
        }
        state.commit_diff.as_ref()
      }
    }
  }

  /// Whether the SCM selection is a merge-conflict entry, which takes no hunk actions.
  fn selection_is_merge(&self, cx: &App) -> bool {
    matches!(self.mode, DiffMode::Scm)
      && self
        .model
        .read(cx)
        .state()
        .selected_file
        .as_ref()
        .is_some_and(|file| file.group_kind == ResourceGroupKind::Merge)
  }

  fn uses_split(&self, cx: &App) -> bool {
    self.effective_mode(AppConfig::get(cx).settings.diff.layout) == ViewMode::Split
      && self.merge_base.is_none()
      && self.split.read(cx).has_document()
  }

  /// Push the layout and inline-change settings into the diff state, then install the loaded payload when it
  /// changed, or drop it when nothing is shown.
  fn sync(&mut self, cx: &mut Context<Self>) {
    let (layout, line_diff) = {
      let diff = &AppConfig::get(cx).settings.diff;
      (diff.layout, diff.line_diff_type)
    };
    enum Plan {
      Skip,
      Clear,
      Apply {
        hash: String,
        merge_base: Option<String>,
        one_sided: bool,
        files: Vec<DiffFile>,
        images: CachedImages,
        split: Option<Box<Document>>,
        language: SharedString,
      },
    }
    let plan = {
      let shown = match &self.mode {
        DiffMode::Scm => self.model.read(cx).state().selected_file.is_some(),
        DiffMode::Commit { path, .. } => !path.is_empty(),
      };
      match (shown, self.ready_payload(cx)) {
        (false, _) => Plan::Clear,
        (true, None) => Plan::Skip,
        (true, Some(payload)) if self.installed.as_deref() == Some(payload.content_hash.as_str()) => Plan::Skip,
        (true, Some(payload)) => {
          let kind = classify(Some(payload));
          let files = if kind == DiffKind::Text {
            parse_files(payload)
          } else {
            Vec::new()
          };
          let images = if kind == DiffKind::Image {
            let (old, new) = states::decode_images(payload);
            CachedImages { old, new }
          } else {
            CachedImages::default()
          };
          let split =
            if kind == DiffKind::Text && payload.merge_text.is_none() && !one_sided(payload) && !files.is_empty() {
              match Document::parse(&payload.path, &payload.patch) {
                Ok(document) => Some(Box::new(document)),
                Err(error) => {
                  tracing::warn!("split diff for {} did not parse: {error}", payload.path);
                  None
                }
              }
            } else {
              None
            };
          let language = files.first().map(DiffFile::language).unwrap_or_else(|| "text".into());
          Plan::Apply {
            hash: payload.content_hash.clone(),
            merge_base: payload.merge_text.as_deref().map(sha256_utf8),
            one_sided: kind == DiffKind::Text && one_sided(payload),
            files,
            images,
            split,
            language,
          }
        }
      }
    };
    match &plan {
      Plan::Skip => {}
      Plan::Clear => self.one_sided = false,
      Plan::Apply { one_sided, .. } => self.one_sided = *one_sided,
    }
    let mode = self.effective_mode(layout);
    let split_anchor = (self.state.read(cx).mode() == ViewMode::Split && mode == ViewMode::Unified)
      .then(|| self.split.read(cx).top_line())
      .flatten();
    self.state.update(cx, |state, cx| {
      if state.mode() != mode {
        state.set_mode(mode, cx);
        if let Some(position) = split_anchor {
          state.scroll_to_line(position, cx);
        }
      }
      if state.inline_unit() != inline_unit(line_diff) {
        state.set_inline_unit(inline_unit(line_diff), cx);
      }
    });
    match plan {
      Plan::Skip => {}
      Plan::Clear => {
        self.merge_base = None;
        if self.installed.take().is_some() {
          self.images = CachedImages::default();
          self.state.update(cx, |state, cx| state.set_files(Vec::new(), cx));
          self
            .split
            .update(cx, |split, cx| split.set_document(None, "text".into(), cx));
        }
      }
      Plan::Apply {
        hash,
        merge_base,
        files,
        images,
        split,
        language,
        ..
      } => {
        self.installed = Some(hash);
        self.merge_base = merge_base;
        self.settle_frames = 2;
        self.images = images;
        self.state.update(cx, |state, cx| state.set_files(files, cx));
        self.split.update(cx, |state, cx| {
          state.set_document(split.map(|document| *document), language, cx)
        });
      }
    }
  }

  /// The layout the viewer paints: the saved choice, except a whole-file add or delete, which has nothing
  /// to put beside its empty side and so shows inline.
  fn effective_mode(&self, layout: DiffLayout) -> ViewMode {
    if self.one_sided {
      ViewMode::Unified
    } else {
      view_mode(layout)
    }
  }

  fn hunk_action(&self, op: HunkOp, hunk_id: String, window: &mut Window, cx: &mut Context<Self>) {
    let intent = hunk_intent(op, hunk_id);
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }

  fn render_hunk_actions(
    hunk: &DiffHunkPayload,
    staged: bool,
    show_range: bool,
    view: WeakEntity<DiffPanel>,
    cx: &App,
  ) -> impl IntoElement {
    let button = |prefix: &str, icon: &'static str, tooltip: &'static str, op: HunkOp| {
      let view = view.clone();
      let hunk_id = hunk.id.clone();
      Button::new(SharedString::from(format!("{prefix}-{}", hunk.id)))
        .ghost()
        .xsmall()
        .icon(Icon::empty().path(icon))
        .tooltip(tooltip)
        .on_click(move |_, window, cx| {
          cx.stop_propagation();
          let _ = view.update(cx, |this, cx| this.hunk_action(op, hunk_id.clone(), window, cx));
        })
    };
    let range = format!(
      "@@ -{},{} +{},{} @@",
      hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines
    );
    // Buttons lead and the range follows: a long line widens the row, and a trailing button would scroll
    // out of reach with it.
    div()
      .flex()
      .items_center()
      .justify_start()
      .gap_2()
      .child(div().flex().flex_shrink_0().items_center().gap_1().children(if staged {
        vec![button(
          "hunk-unstage",
          "icons/minus.svg",
          "Unstage Hunk",
          HunkOp::Unstage,
        )]
      } else {
        vec![
          button("hunk-stage", "icons/plus.svg", "Stage Hunk", HunkOp::Stage),
          button("hunk-discard", "icons/undo-2.svg", "Discard Hunk", HunkOp::Discard),
        ]
      }))
      .child(
        div()
          .min_w_0()
          .truncate()
          .text_xs()
          .text_color(cx.theme().muted_foreground)
          .child(if show_range { range } else { String::new() }),
      )
  }
}

impl Render for DiffPanel {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.sync(cx);
    if self.settle_frames > 0 {
      self.settle_frames -= 1;
      let view = cx.weak_entity();
      window.on_next_frame(move |_, cx| {
        let _ = view.update(cx, |_, cx| cx.notify());
      });
    }
    let palette = cx.global::<ActivePalette>().0;
    let (layout, show_line_numbers, show_background, indicators, separators, wrap, show_hunk_actions) = {
      let settings = &AppConfig::get(cx).settings;
      (
        settings.diff.layout,
        settings.diff.show_line_numbers,
        settings.diff.show_background,
        settings.diff.diff_indicators,
        settings.diff.hunk_separators,
        settings.editor.word_wrap == WordWrap::On,
        settings.diff.show_inline_hunk_actions,
      )
    };
    let is_scm = matches!(self.mode, DiffMode::Scm);
    let commit_mode = match &self.mode {
      DiffMode::Commit { commit, path, status } => Some((commit.clone(), path.clone(), status.clone())),
      DiffMode::Scm => None,
    };
    let merge = self.selection_is_merge(cx);
    let (selected, kind, load_ready, has_repository, staged, hunks, path) = {
      let model_state = self.model.read(cx).state();
      let payload = self.ready_payload(cx);
      (
        model_state.selected_file.clone(),
        classify(payload),
        payload.is_some(),
        model_state.has_repository(),
        payload.is_some_and(|payload| payload.staged),
        payload.map(|payload| payload.hunks.clone()).unwrap_or_default(),
        payload.map(|payload| payload.path.clone()).unwrap_or_default(),
      )
    };
    let show_nav = load_ready && kind == DiffKind::Text;
    // A whole-file add or delete always shows inline, so the layout control would do nothing.
    let show_layout = !(load_ready && self.one_sided);
    let weak = cx.weak_entity();
    let mut root = div()
      .id("diff-panel")
      .role(Role::Pane)
      .size_full()
      .min_w_0()
      .flex()
      .flex_col()
      .on_action(cx.listener(|this, _: &ClearSelection, window, cx| {
        if this.clear_selection(window, cx) {
          cx.stop_propagation();
          cx.notify();
        }
      }))
      .on_action(cx.listener(|this, _: &NextDiffChange, _, cx| this.next_change(cx)))
      .on_action(cx.listener(|this, _: &PreviousDiffChange, _, cx| this.previous_change(cx)));
    if let Some((_commit, path, status)) = commit_mode {
      if path.is_empty() {
        return root.child(div().flex_1().min_h_0());
      }
      root = root.child(header::render_commit_header(
        &path,
        status,
        layout,
        show_nav,
        show_layout,
        weak.clone(),
        palette,
      ));
    } else {
      let Some(selection) = selected else {
        return root.child(states::render_empty(has_repository, palette));
      };
      root = root.child(header::render_header(
        &selection,
        layout,
        show_nav,
        show_layout,
        weak.clone(),
        palette,
        cx,
      ));
    }
    if !load_ready {
      return root.child(div().flex_1().min_h_0());
    }
    match kind {
      DiffKind::Empty => root.child(div().flex_1().min_h_0()),
      DiffKind::Image => root.child(states::render_image(
        self.images.old.clone(),
        self.images.new.clone(),
        palette,
      )),
      DiffKind::Binary => root.child(states::render_binary(weak, palette)),
      DiffKind::Large => root.child(states::render_large(weak, palette)),
      DiffKind::Text => {
        let file_path = self
          .state
          .read(cx)
          .files()
          .first()
          .map(|file| file.path().to_string())
          .unwrap_or(path);
        let actions = show_hunk_actions && is_scm && !merge;
        if self.uses_split(cx) {
          let show_range = separators != HunkSeparators::Metadata;
          let render = actions.then(|| {
            Rc::new(move |target: &Hunk, _: &mut Window, cx: &mut App| {
              div()
                .children(
                  hunks
                    .iter()
                    .find(|hunk| {
                      (hunk.old_start, hunk.old_lines, hunk.new_start, hunk.new_lines)
                        == (target.old_start, target.old_lines, target.new_start, target.new_lines)
                    })
                    .map(|hunk| Self::render_hunk_actions(hunk, staged, show_range, weak.clone(), cx)),
                )
                .into_any_element()
            }) as Rc<dyn Fn(&Hunk, &mut Window, &mut App) -> AnyElement>
          });
          return root.child(SplitDiff::new(&self.split, render));
        }
        let (annotations, targets) = if actions {
          hunk_annotations(&file_path, &hunks)
        } else {
          (Vec::new(), HashMap::new())
        };
        let targets = Rc::new(targets);
        // The Metadata separator already prints each hunk's range.
        let show_range = separators != HunkSeparators::Metadata;
        let view = weak.clone();
        root.child(
          Diff::new(&self.state)
            .flex_1()
            .min_h_0()
            .border_0()
            .header_visible(false)
            .line_number(show_line_numbers)
            .change_background(show_background)
            .change_indicator(change_indicator(indicators))
            .hunk_separator(hunk_separator(separators))
            .soft_wrap(wrap)
            .annotations(annotations)
            .render_annotation(move |annotation, _, cx| {
              let hunk = annotation.position().and_then(|position| targets.get(position));
              div().children(hunk.map(|hunk| Self::render_hunk_actions(hunk, staged, show_range, view.clone(), cx)))
            }),
        )
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use deathpush_core::Core;
  use deathpush_core::session::types::{
    FileSelection, OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot,
    SyncAction, SyncKind,
  };
  use deathpush_core::types::{RepoOperationState, StatusPhase};
  use gpui_kit::TestAppContext;
  use gpui_kit::component::diff::DiffConflictResolution;

  use crate::config::AppConfig;
  use crate::repo::layout_model::LayoutModel;

  const PATCH: &str = "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,2 +1,3 @@\n fn main() {}\n+let x = 1;\n fn end() {}\n@@ -10,2 +11,1 @@\n-gone();\n keep();\n";

  fn hunk(id: &str, old: (usize, usize), new: (usize, usize)) -> DiffHunkPayload {
    DiffHunkPayload {
      id: id.into(),
      old_start: old.0,
      old_lines: old.1,
      new_start: new.0,
      new_lines: new.1,
    }
  }

  #[test]
  fn hunk_buttons_map_to_hunk_intents() {
    assert!(matches!(
      hunk_intent(HunkOp::Stage, "a".into()),
      Intent::StageHunk { hunk_id } if hunk_id == "a"
    ));
    assert!(matches!(
      hunk_intent(HunkOp::Unstage, "a".into()),
      Intent::UnstageHunk { hunk_id } if hunk_id == "a"
    ));
    assert!(matches!(
      hunk_intent(HunkOp::Discard, "a".into()),
      Intent::DiscardHunk { hunk_id, confirmed: false } if hunk_id == "a"
    ));
  }

  #[test]
  fn hunk_rows_hang_under_the_first_old_line_unless_the_hunk_has_none() {
    let hunks = vec![
      hunk("edit", (1, 2), (1, 3)),
      hunk("del", (10, 2), (0, 0)),
      hunk("new", (0, 0), (1, 3)),
    ];
    let (annotations, targets) = hunk_annotations("src/main.rs", &hunks);
    assert_eq!(annotations.len(), 3);
    let first = annotations[0].position().unwrap();
    assert_eq!((first.side(), first.line()), (DiffSide::Original, 1));
    let second = annotations[1].position().unwrap();
    assert_eq!((second.side(), second.line()), (DiffSide::Original, 10));
    let third = annotations[2].position().unwrap();
    assert_eq!((third.side(), third.line()), (DiffSide::Modified, 1));
    assert_eq!(targets[first].id, "edit");
    assert_eq!(targets[second].id, "del");
    assert_eq!(targets[third].id, "new");
  }

  #[test]
  fn whole_file_adds_and_deletes_are_one_sided() {
    let with_hunks = |hunks: Vec<DiffHunkPayload>| {
      let mut payload = text_payload(PATCH, "rust");
      payload.hunks = hunks;
      payload
    };
    assert!(one_sided(&with_hunks(vec![hunk("add", (0, 0), (1, 2))])));
    assert!(one_sided(&with_hunks(vec![hunk("del", (1, 2), (0, 0))])));
    assert!(!one_sided(&with_hunks(vec![hunk("edit", (1, 2), (1, 3))])));
    assert!(!one_sided(&with_hunks(vec![
      hunk("add", (0, 0), (1, 2)),
      hunk("edit", (5, 2), (6, 3)),
    ])));
    assert!(!one_sided(&with_hunks(Vec::new())));
    let mut merge = with_hunks(vec![hunk("add", (0, 0), (1, 2))]);
    merge.merge_text = Some("text".into());
    assert!(!one_sided(&merge));
  }

  #[test]
  fn settings_map_to_the_viewer() {
    assert_eq!(view_mode(DiffLayout::SideBySide), ViewMode::Split);
    assert_eq!(view_mode(DiffLayout::Inline), ViewMode::Unified);
    assert_eq!(inline_unit(LineDiffType::Word), Some(DiffInlineUnit::Word));
    assert_eq!(inline_unit(LineDiffType::Char), Some(DiffInlineUnit::Character));
    assert_eq!(inline_unit(LineDiffType::None), None);
  }

  #[test]
  fn payload_language_reaches_the_parsed_file() {
    let payload = text_payload(PATCH, "rust");
    let files = parse_files(&payload);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path().as_str(), "src/main.rs");
    assert_eq!(files[0].language().as_str(), "rust");
    assert_eq!((files[0].additions(), files[0].deletions()), (1, 1));
  }

  #[gpui_kit::test]
  fn a_merge_file_offers_its_conflicts_as_choices(cx: &mut TestAppContext) {
    let mut payload = text_payload("", "rust");
    payload.path = "c.txt".into();
    payload.merge_text = Some("one\n<<<<<<< HEAD\nmine\n=======\ntheirs\n>>>>>>> other\nthree\n".into());
    let files = parse_files(&payload);
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].conflicts().len(), 1);
    let state = cx.new(|cx| DiffState::new(files, cx));
    state.update(cx, |state, cx| {
      assert_eq!(
        state.resolved_text("c.txt"),
        None,
        "an unresolved conflict has no resolved text"
      );
      state.resolve_conflict("c.txt", 0, Some(DiffConflictResolution::Incoming), cx);
      assert_eq!(state.resolved_text("c.txt").as_deref(), Some("one\ntheirs\nthree\n"));
    });
  }

  #[test]
  fn a_patch_that_does_not_parse_shows_nothing() {
    let payload = text_payload("@@ -x +y @@\n", "rust");
    assert!(parse_files(&payload).is_empty());
  }

  fn snapshot(root: &str, file: Option<FileSelection>) -> SessionSnapshot {
    SessionSnapshot {
      session_generation: 1,
      session_revision: 1,
      status_generation: 1,
      status_revision: 1,
      repo: SessionRepo {
        root: root.into(),
        has_repository: true,
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

  fn text_payload(patch: &str, language: &str) -> DiffPayload {
    DiffPayload {
      path: "src/main.rs".into(),
      patch: patch.to_string(),
      merge_text: None,
      old_image: String::new(),
      new_image: String::new(),
      language: Some(language.into()),
      file_type: "text".into(),
      hunks: vec![hunk("first", (1, 2), (1, 3)), hunk("second", (10, 2), (11, 1))],
      staged: false,
      content_hash: format!("hash-{}", patch.len()),
    }
  }

  #[gpui_kit::test]
  fn a_loaded_text_diff_reaches_the_viewer_in_the_chosen_layout(cx: &mut TestAppContext) {
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
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        DiffPanel::new(model, layout, window, cx)
      }
    });
    window
      .update(cx, |panel, window, cx| {
        panel.model().update(cx, |model, _| {
          let load_id = model.state().selected_load_id;
          model.state_mut().diff = Some(text_payload(PATCH, "rust"));
          model.state_mut().diff_load_id = Some(load_id);
        });
        window.refresh();
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |panel, _, cx| {
        let state = panel.state().read(cx);
        assert_eq!(state.files().len(), 1);
        assert_eq!(state.files()[0].path().as_str(), "src/main.rs");
        assert_eq!(state.mode(), ViewMode::Split);
      })
      .unwrap();
    cx.update(|cx| AppConfig::update(cx, |config| config.settings.diff.layout = DiffLayout::Inline));
    cx.run_until_parked();
    window
      .update(cx, |panel, _, cx| {
        assert_eq!(panel.state().read(cx).mode(), ViewMode::Unified);
      })
      .unwrap();

    crate::test_core::park_and_shutdown(cx, &core);
  }

  #[gpui_kit::test]
  fn f7_moves_a_focused_side_by_side_diff_to_the_next_change(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      let bindings = crate::keymap::bindings(cx);
      cx.bind_keys(bindings);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (session, _events) = core.open_session();
    let layout_dir = config_dir.path().to_path_buf();
    let root = layout_dir.to_string_lossy().into_owned();
    let (panel, cx) = cx.add_window_view({
      let core = core.clone();
      let snapshot = snapshot(
        &root,
        Some(FileSelection {
          path: "src/main.rs".into(),
          staged: false,
          group_kind: ResourceGroupKind::WorkingTree,
        }),
      );
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        DiffPanel::new(model, layout, window, cx)
      }
    });
    // Each hunk is taller than the window, so moving to the second change has to scroll.
    let context: String = (1..=100).map(|n| format!(" ctx{n}\n")).collect();
    let patch = format!(
      "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,101 +1,101 @@\n-old\n+new\n{context}@@ -300,101 +300,101 @@\n-far\n+near\n{context}"
    );
    panel.update_in(cx, |panel, window, cx| {
      panel.model().update(cx, |model, _| {
        let load_id = model.state().selected_load_id;
        model.state_mut().diff = Some(text_payload(&patch, "rust"));
        model.state_mut().diff_load_id = Some(load_id);
      });
      window.refresh();
    });
    cx.run_until_parked();
    let top = |cx: &mut gpui_kit::VisualTestContext| {
      cx.run_until_parked();
      cx.update(|window, cx| {
        let _ = window.draw(cx);
      });
      panel.update(cx, |panel, cx| panel.split.read(cx).top_line().map(|line| line.line()))
    };
    assert_eq!(top(cx), Some(1));
    panel.update_in(cx, |panel, window, cx| {
      let focus = panel.split.read(cx).focus_handle();
      window.focus(&focus, cx);
    });
    cx.simulate_keystrokes("f7");
    assert_eq!(top(cx), Some(1));
    cx.simulate_keystrokes("f7");
    assert_eq!(top(cx), Some(300));
    cx.simulate_keystrokes("shift-f7");
    assert_eq!(top(cx), Some(1));

    crate::test_core::park_and_shutdown(cx, &core);
  }
}
