use deathpush_core::session::types::Intent;
use deathpush_core::types::FileStatus;
use gpui_kit::base::{ResizableState, h_resizable, resizable_panel};
use gpui_kit::*;

use super::detail;
use super::list;
use crate::repo::diff::{DiffMode, DiffPanel};
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::{ActivePalette, hsla};

/// The History main panel: commit list, detail, and a commit-mode DiffPanel.
pub struct HistoryView {
  repo: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  diff: Entity<DiffPanel>,
  files_as_tree: bool,
  split_state: Entity<ResizableState>,
  focus_handle: FocusHandle,
}

impl HistoryView {
  /// Build the History split. The given DiffPanel is switched to Commit mode.
  pub fn new(
    repo: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    diff: Entity<DiffPanel>,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&repo, |_, _, cx| cx.notify()).detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    diff.update(cx, |panel, cx| {
      panel.set_mode(
        DiffMode::Commit {
          commit: String::new(),
          path: String::new(),
          status: FileStatus::Modified,
        },
        cx,
      );
    });
    Self {
      repo,
      layout,
      diff,
      files_as_tree: false,
      split_state: cx.new(|_| ResizableState::default()),
      focus_handle: cx.focus_handle(),
    }
  }

  /// Flip the changed-files list between a flat list and a nested tree.
  pub fn toggle_files_as_tree(&mut self, cx: &mut Context<Self>) {
    self.files_as_tree = !self.files_as_tree;
    cx.notify();
  }

  pub(crate) fn select_commit(&mut self, id: String, window: &mut Window, cx: &mut Context<Self>) {
    let current = self.repo.read(cx).state().selected_commit.clone();
    if current.as_deref() == Some(id.as_str()) {
      return;
    }
    self.diff.update(cx, |panel, cx| {
      panel.set_mode(
        DiffMode::Commit {
          commit: id.clone(),
          path: String::new(),
          status: FileStatus::Modified,
        },
        cx,
      );
    });
    self.repo.update(cx, |model, cx| model.select_commit(id, window, cx));
  }

  pub(crate) fn load_more(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.repo.update(cx, |model, cx| model.load_more_log(window, cx));
  }

  pub(crate) fn open_commit_file(
    &mut self,
    commit: String,
    path: String,
    status: FileStatus,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    self.diff.update(cx, |panel, cx| {
      panel.set_mode(
        DiffMode::Commit {
          commit: commit.clone(),
          path: path.clone(),
          status,
        },
        cx,
      );
    });
    self
      .repo
      .update(cx, |model, cx| model.open_commit_diff(commit, path, window, cx));
    cx.notify();
  }

  pub(crate) fn clear_file_history(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.repo.update(cx, |model, cx| model.clear_file_history(window, cx));
  }

  pub(crate) fn copy(&self, text: String, cx: &mut App) {
    cx.write_to_clipboard(ClipboardItem::new_string(text));
  }

  pub(crate) fn cherry_pick(&mut self, commit: String, window: &mut Window, cx: &mut Context<Self>) {
    self.repo.update(cx, |model, cx| {
      model.dispatch(Intent::CherryPick { commit }, window, cx)
    });
  }

  pub(crate) fn reset(&mut self, commit: String, mode: String, window: &mut Window, cx: &mut Context<Self>) {
    self.repo.update(cx, |model, cx| {
      model.dispatch(
        Intent::Reset {
          commit,
          mode,
          confirmed: false,
        },
        window,
        cx,
      );
    });
  }
}

impl Render for HistoryView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let list_width = self.layout.read(cx).layout().history_list_width;
    let (log, selected, file_history, entry, files) = {
      let state = self.repo.read(cx).state();
      let selected = state.selected_commit.clone();
      let entry = selected.as_ref().and_then(|id| {
        state
          .commit_detail
          .as_ref()
          .filter(|detail| &detail.commit.id == id)
          .map(|detail| detail.commit.clone())
          .or_else(|| state.commit_log.iter().find(|commit| &commit.id == id).cloned())
      });
      let files = state
        .commit_detail
        .as_ref()
        .filter(|detail| selected.as_ref() == Some(&detail.commit.id))
        .map(|detail| detail.files.clone())
        .unwrap_or_default();
      (
        state.commit_log.clone(),
        selected,
        state.file_history_path.clone(),
        entry,
        files,
      )
    };
    let selected_file = match self.diff.read(cx).mode() {
      DiffMode::Commit { path, .. } if !path.is_empty() => Some(path.clone()),
      _ => None,
    };
    let view = cx.weak_entity();
    let list = list::render_list(
      &log,
      selected.as_deref(),
      file_history.as_deref(),
      view.clone(),
      palette,
    )
    .into_any_element();
    let mut detail = div().size_full().flex().flex_col();
    detail = match entry {
      Some(entry) => {
        let commit_id = entry.id.clone();
        detail
          .child(detail::render_header(&entry, view.clone(), palette))
          .child(detail::render_files(
            &files,
            self.files_as_tree,
            selected_file.as_deref(),
            &commit_id,
            view,
            palette,
          ))
          .child(self.diff.clone().into_any_element())
      }
      None => detail.child(detail::render_empty(palette)),
    };
    let layout_entity = self.layout.clone();
    let split = h_resizable("history-split")
      .with_state(&self.split_state)
      .on_resize(move |state, _, cx| {
        if let Some(width) = state.read(cx).sizes().first().copied() {
          layout_entity.update(cx, |layout, cx| layout.set_history_list_width(f32::from(width), cx));
        }
      })
      .child(
        resizable_panel()
          .size(px(list_width))
          .size_range(px(200.0)..px(600.0))
          .flex_none()
          .child(list),
      )
      .child(resizable_panel().child(detail));
    div()
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .bg(hsla(palette.background))
      .child(split)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  use deathpush_core::Core;
  use deathpush_core::session::types::{
    OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{CommitEntry, RepoOperationState, StatusPhase};
  use gpui_kit::{TestAppContext, WindowHandle};

  use crate::config::AppConfig;
  use crate::repo::layout_model::LayoutModel;
  use crate::repo::model::RepoModel;

  fn snapshot(root: &str, log: Vec<CommitEntry>) -> SessionSnapshot {
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
      selection: SessionSelection::default(),
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
      commit_log: log,
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  fn commit() -> CommitEntry {
    CommitEntry {
      id: "0123456789abcdef0123456789abcdef01234567".into(),
      short_id: "0123456".into(),
      message: "add the history view".into(),
      author_name: "Ana Lima".into(),
      author_email: "ana@example.com".into(),
      author_date: "2026-09-01T00:00:00Z".into(),
      parent_ids: vec![],
      avatar_url: String::new(),
    }
  }

  fn open_history(cx: &mut TestAppContext, log: Vec<CommitEntry>) -> WindowHandle<HistoryView> {
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
    cx.add_window({
      let core = core.clone();
      let snapshot = snapshot(&root, log);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |_, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        let diff = cx.new(|cx| DiffPanel::new(model.clone(), layout.clone(), cx));
        HistoryView::new(model, layout, diff, cx)
      }
    })
  }

  #[gpui_kit::test]
  fn files_toggle_flips(cx: &mut TestAppContext) {
    let window = open_history(cx, vec![commit()]);
    window
      .update(cx, |view, window, cx| {
        window.refresh();
        assert!(!view.files_as_tree);
        view.toggle_files_as_tree(cx);
        assert!(view.files_as_tree);
        view.toggle_files_as_tree(cx);
        assert!(!view.files_as_tree);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn history_view_renders_with_a_log(cx: &mut TestAppContext) {
    let window = open_history(cx, vec![commit()]);
    window
      .update(cx, |view, window, cx| {
        view.repo.update(cx, |model, _| {
          model.state_mut().apply_snapshot(snapshot("/tmp/repo", vec![commit()]));
        });
        window.refresh();
        assert_eq!(view.repo.read(cx).state().commit_log.len(), 1);
        assert_eq!(view.repo.read(cx).state().commit_log[0].short_id, "0123456");
      })
      .unwrap();
  }
}
