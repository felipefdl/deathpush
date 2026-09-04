use deathpush_core::config::layout::{MainView, PanelTab, SidebarView};
use deathpush_core::config::settings::{DiffLayout, SidebarPosition};
use deathpush_core::session::types::Intent;
use gpui_kit::base::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::changes::ChangesView;
use super::diff::DiffPanel;
use super::layout_model::LayoutModel;
use super::main_panel::render_main_panel;
use super::model::RepoModel;
use super::output_log::OutputLog;
use super::sidebar::render_sidebar;
use super::state::NetworkOp;
use super::status_bar::render_status_bar;
use super::terminal_panel::render_terminal_panel;
use crate::actions::*;
use crate::config::AppConfig;
use crate::theme::{ActivePalette, hsla};

/// The repository window chrome from docs/specs/app-shell.md.
pub struct RepoView {
  model: Entity<RepoModel>,
  layout: Entity<LayoutModel>,
  output: Entity<OutputLog>,
  changes: Entity<ChangesView>,
  diff: Entity<DiffPanel>,
  body_state: Entity<ResizableState>,
  main_state: Entity<ResizableState>,
  pub(crate) focus_handle: FocusHandle,
}

impl RepoView {
  pub fn new(
    model: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    output: Entity<OutputLog>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&model, |_, _, cx| cx.notify()).detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    cx.observe(&output, |_, _, cx| cx.notify()).detach();
    let changes = cx.new(|cx| ChangesView::new(model.clone(), layout.clone(), window, cx));
    let diff = cx.new(|cx| DiffPanel::new(model.clone(), layout.clone(), cx));
    Self {
      model,
      layout,
      output,
      changes,
      diff,
      body_state: cx.new(|_| ResizableState::default()),
      main_state: cx.new(|_| ResizableState::default()),
      focus_handle: cx.focus_handle(),
    }
  }

  pub fn model(&self) -> &Entity<RepoModel> {
    &self.model
  }

  #[allow(dead_code)]
  pub fn layout(&self) -> &Entity<LayoutModel> {
    &self.layout
  }

  pub fn output(&self) -> &Entity<OutputLog> {
    &self.output
  }

  #[allow(dead_code)]
  pub fn changes(&self) -> &Entity<ChangesView> {
    &self.changes
  }

  #[allow(dead_code)]
  pub fn diff(&self) -> &Entity<DiffPanel> {
    &self.diff
  }

  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  fn send(&self, intent: Intent, window: &mut Window, cx: &mut Context<Self>) {
    self.model.update(cx, |model, cx| model.dispatch(intent, window, cx));
  }

  fn show_terminal_tab(&self, cx: &mut Context<Self>) {
    self.layout.update(cx, |layout, cx| {
      layout.set_terminal_visible(true, cx);
      layout.set_panel_tab(PanelTab::Terminal, cx);
    });
  }

  fn render_body(&self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let layout = self.layout.read(cx).layout().clone();
    let sidebar_right = AppConfig::get(cx).settings.ui.sidebar_position == SidebarPosition::Right;
    let layout_entity = self.layout.clone();
    let select = move |view: SidebarView, _: &mut Window, cx: &mut App| {
      layout_entity.update(cx, |layout, cx| layout.select_sidebar_view(view, cx));
    };
    let sidebar_body = if layout.sidebar_view == SidebarView::Scm {
      self.changes.clone().into_any_element()
    } else {
      div().into_any_element()
    };
    let sidebar = render_sidebar(layout.sidebar_view, select, sidebar_body, cx).into_any_element();
    let main_panel = render_main_panel(layout.main_view, &self.diff, cx).into_any_element();
    let terminal =
      render_terminal_panel(layout.panel_tab, layout.terminal_maximized, &self.output, cx).into_any_element();
    let main_area: AnyElement = match (layout.terminal_visible, layout.terminal_maximized) {
      (false, _) => main_panel,
      (true, true) => terminal,
      (true, false) => {
        let layout_entity = self.layout.clone();
        v_resizable("main-area")
          .with_state(&self.main_state)
          .on_resize(move |state, _, cx| {
            if let Some(height) = state.read(cx).sizes().get(1).copied() {
              layout_entity.update(cx, |layout, cx| layout.set_terminal_height(f32::from(height), cx));
            }
          })
          .child(resizable_panel().child(main_panel))
          .child(
            resizable_panel()
              .size(px(layout.terminal_height))
              .size_range(px(100.0)..px(600.0))
              .flex_none()
              .child(terminal),
          )
          .into_any_element()
      }
    };
    let layout_entity = self.layout.clone();
    let sidebar_index = if sidebar_right { 1 } else { 0 };
    let sidebar_panel = resizable_panel()
      .size(px(layout.sidebar_width))
      .size_range(px(200.0)..px(600.0))
      .flex_none()
      .child(sidebar);
    let mut group = h_resizable("shell-body")
      .with_state(&self.body_state)
      .on_resize(move |state, _, cx| {
        if let Some(width) = state.read(cx).sizes().get(sidebar_index).copied() {
          layout_entity.update(cx, |layout, cx| layout.set_sidebar_width(f32::from(width), cx));
        }
      });
    group = if sidebar_right {
      group.child(resizable_panel().child(main_area)).child(sidebar_panel)
    } else {
      group.child(sidebar_panel).child(resizable_panel().child(main_area))
    };
    let _ = window;
    div().flex_1().min_h_0().child(group)
  }
}

impl Render for RepoView {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let status_bar = {
      let model = self.model.read(cx);
      render_status_bar(model.state(), window, cx).into_any_element()
    };
    let body = self.render_body(window, cx).into_any_element();
    div()
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .flex_col()
      .bg(hsla(palette.background))
      .on_action(cx.listener(|this, _: &ShowChanges, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.select_sidebar_view(SidebarView::Scm, cx));
      }))
      .on_action(cx.listener(|this, _: &ShowExplorer, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.select_sidebar_view(SidebarView::Explorer, cx));
      }))
      .on_action(cx.listener(|this, _: &ShowHistory, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.select_main_view(MainView::History, cx));
      }))
      .on_action(cx.listener(|this, _: &ShowSettings, _, cx| {
        this.layout.update(cx, |layout, cx| {
          let next = if layout.layout().main_view == MainView::Settings {
            MainView::Changes
          } else {
            MainView::Settings
          };
          layout.select_main_view(next, cx);
        });
      }))
      .on_action(cx.listener(|_, _: &ToggleDiffLayout, _, cx| {
        AppConfig::update(cx, |config| {
          config.settings.diff.layout = match config.settings.diff.layout {
            DiffLayout::Inline => DiffLayout::SideBySide,
            DiffLayout::SideBySide => DiffLayout::Inline,
          };
        });
        cx.notify();
      }))
      .on_action(cx.listener(|this, _: &ReloadSession, window, cx| {
        this.model.update(cx, |model, cx| model.reload(window, cx));
      }))
      .on_action(cx.listener(|_, _: &SwallowSave, _, _| {}))
      .on_action(cx.listener(|this, _: &ClearSelection, window, cx| {
        if !window.has_focused_input(cx) {
          this.send(Intent::ClearFile, window, cx);
        }
      }))
      .on_action(cx.listener(|this, _: &ToggleTerminal, _, cx| {
        this.layout.update(cx, |layout, cx| {
          let visible = !layout.layout().terminal_visible;
          layout.set_terminal_visible(visible, cx);
        });
      }))
      .on_action(cx.listener(|this, _: &FocusTerminal, _, cx| this.show_terminal_tab(cx)))
      .on_action(cx.listener(|this, _: &NewTerminal, _, cx| this.show_terminal_tab(cx)))
      .on_action(cx.listener(|this, _: &ShowOutputTab, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.set_panel_tab(PanelTab::GitOutput, cx));
      }))
      .on_action(cx.listener(|this, _: &ShowTerminalTab, _, cx| this.show_terminal_tab(cx)))
      .on_action(cx.listener(|this, _: &ToggleTerminalMaximize, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.toggle_terminal_maximized(cx));
      }))
      .on_action(cx.listener(|this, _: &ClosePanel, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.set_terminal_visible(false, cx));
      }))
      .on_action(cx.listener(|this, _: &GitPull, window, cx| {
        this.model.update(cx, |model, cx| {
          model.dispatch_network(NetworkOp::Pull, Intent::Pull { rebase: false }, window, cx);
        });
      }))
      .on_action(cx.listener(|this, _: &GitPush, window, cx| {
        this.model.update(cx, |model, cx| {
          model.dispatch_network(
            NetworkOp::Push,
            Intent::Push {
              force: false,
              confirmed: false,
            },
            window,
            cx,
          );
        });
      }))
      .on_action(cx.listener(|this, _: &GitFetch, window, cx| {
        this.model.update(cx, |model, cx| {
          model.dispatch_network(NetworkOp::Fetch, Intent::Fetch { prune: true }, window, cx);
        });
      }))
      .on_action(cx.listener(|this, _: &GitStageAll, window, cx| this.send(Intent::StageAll, window, cx)))
      .on_action(cx.listener(|this, _: &GitUnstageAll, window, cx| this.send(Intent::UnstageAll, window, cx)))
      .on_action(cx.listener(|this, _: &GitStash, window, cx| {
        this.send(
          Intent::StashSave {
            include_untracked: false,
            staged_only: false,
            message: None,
          },
          window,
          cx,
        )
      }))
      .on_action(cx.listener(|this, _: &GitStashPop, window, cx| this.send(Intent::StashPop { index: 0 }, window, cx)))
      .on_action(
        cx.listener(|this, _: &GitUndoCommit, window, cx| {
          this.send(Intent::UndoCommit { confirmed: false }, window, cx)
        }),
      )
      .child(body)
      .child(status_bar)
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
  use deathpush_core::types::{
    FileEntry, FileStatus, RepoOperationState, ResourceGroup, ResourceGroupKind, StatusPhase,
  };
  use gpui_kit::TestAppContext;

  use crate::config::AppConfig;
  use crate::repo::layout_model::LayoutModel;
  use crate::repo::model::RepoModel;
  use crate::repo::output_log::OutputLog;

  fn snapshot(root: &str) -> SessionSnapshot {
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
      commit_log: vec![],
      commit_detail: None,
      file_history_path: None,
      error: None,
    }
  }

  #[gpui_kit::test]
  fn repo_view_owns_focus_after_focus(cx: &mut TestAppContext) {
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
      let snapshot = snapshot(&root);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        let output = cx.new(|_| OutputLog::default());
        RepoView::new(model, layout, output, window, cx)
      }
    });
    window
      .update(cx, |view, window, cx| {
        view.focus(window, cx);
        assert_eq!(window.focused(cx).as_ref(), Some(&view.focus_handle));
      })
      .unwrap();
  }

  fn populated_snapshot(root: &str) -> SessionSnapshot {
    let mut snapshot = snapshot(root);
    snapshot.groups = vec![
      ResourceGroup {
        kind: ResourceGroupKind::Index,
        label: "Staged Changes".into(),
        files: vec![FileEntry {
          path: "a.rs".into(),
          status: FileStatus::IndexModified,
          rename_path: None,
        }],
      },
      ResourceGroup {
        kind: ResourceGroupKind::WorkingTree,
        label: "Changes".into(),
        files: vec![FileEntry {
          path: "b.rs".into(),
          status: FileStatus::Modified,
          rename_path: None,
        }],
      },
    ];
    snapshot
  }

  #[gpui_kit::test]
  fn changes_view_renders_with_status_groups(cx: &mut TestAppContext) {
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
      let snapshot = snapshot(&root);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        let output = cx.new(|_| OutputLog::default());
        RepoView::new(model, layout, output, window, cx)
      }
    });
    window
      .update(cx, |view, window, cx| {
        view.model().update(cx, |model, _| {
          model.state_mut().apply_snapshot(populated_snapshot(&root));
        });
        window.refresh();
        assert!(view.changes().read(cx).filter_text().is_empty());
        assert!(view.model().read(cx).state().has_changes());
      })
      .unwrap();
  }
}
