use deathpush_core::config::layout::{MainView, PanelTab, SidebarView};
use deathpush_core::config::settings::{DiffLayout, SidebarPosition};
use deathpush_core::session::types::Intent;
use gpui_kit::base::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::changes::ChangesView;
use super::changes::overflow::{OverflowItem, dispatch_item};
use super::diff::DiffPanel;
use super::explorer::{ExplorerModel, ExplorerView};
use super::file_viewer::FileViewer;
use super::history::HistoryView;
use super::layout_model::LayoutModel;
use super::main_panel::render_main_panel;
use super::model::RepoModel;
use super::output_log::OutputLog;
use super::settings::SettingsView;
use super::sidebar::render_sidebar;
use super::state::NetworkOp;
use super::status_bar::render_status_bar;
use super::terminal::model::TerminalModel;
use super::terminal::panel::TerminalPanel;
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
  file: Entity<FileViewer>,
  history: Entity<HistoryView>,
  settings: Entity<SettingsView>,
  explorer_model: Entity<ExplorerModel>,
  explorer: Entity<ExplorerView>,
  terminal: Entity<TerminalModel>,
  terminal_panel: Entity<TerminalPanel>,
  body_state: Entity<ResizableState>,
  main_state: Entity<ResizableState>,
  pub(crate) focus_handle: FocusHandle,
  settings_restore: Option<WeakFocusHandle>,
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
    let (core, session, root) = {
      let model = model.read(cx);
      (
        model.core(),
        model.session(),
        model.state().root().unwrap_or("").to_string(),
      )
    };
    let explorer_model = cx.new({
      let core = core.clone();
      let root = root.clone();
      move |cx| {
        let mut explorer = ExplorerModel::new(core, session, root);
        explorer.load(cx);
        explorer
      }
    });
    cx.observe(&explorer_model, |_, _, cx| cx.notify()).detach();
    let explorer = cx.new(|cx| ExplorerView::new(explorer_model.clone(), model.clone(), layout.clone(), window, cx));
    let changes = cx.new(|cx| ChangesView::new(model.clone(), layout.clone(), window, cx));
    let diff = cx.new(|cx| DiffPanel::new(model.clone(), layout.clone(), cx));
    let file = cx.new(|cx| FileViewer::new(model.clone(), layout.clone(), window, cx));
    let history_diff = cx.new(|cx| DiffPanel::new(model.clone(), layout.clone(), cx));
    let history = cx.new(|cx| HistoryView::new(model.clone(), layout.clone(), history_diff, cx));
    let settings = cx.new(|cx| SettingsView::new(model.clone(), layout.clone(), core.clone(), window, cx));
    cx.observe(&settings, |_, _, cx| cx.notify()).detach();
    let terminal = cx.new({
      let core = core.clone();
      move |cx| TerminalModel::new(core, session, cx)
    });
    cx.observe(&terminal, |_, _, cx| cx.notify()).detach();
    if layout.read(cx).layout().terminal_visible {
      terminal.update(cx, |model, cx| {
        model.ensure_group(window, cx);
      });
    }
    let terminal_panel = cx.new(|cx| TerminalPanel::new(terminal.clone(), layout.clone(), output.clone(), cx));
    Self {
      model,
      layout,
      output,
      changes,
      diff,
      file,
      history,
      settings,
      explorer_model,
      explorer,
      terminal,
      terminal_panel,
      body_state: cx.new(|_| ResizableState::default()),
      main_state: cx.new(|_| ResizableState::default()),
      focus_handle: cx.focus_handle(),
      settings_restore: None,
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

  pub fn explorer_model(&self) -> &Entity<ExplorerModel> {
    &self.explorer_model
  }

  pub fn explorer(&self) -> &Entity<ExplorerView> {
    &self.explorer
  }

  pub fn terminal(&self) -> &Entity<TerminalModel> {
    &self.terminal
  }

  fn activate_terminal_group(&self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
    let activated = self.terminal.update(cx, |model, cx| model.activate_group(index, cx));
    if !activated {
      return;
    }
    self.show_terminal_tab(cx);
    self.terminal.update(cx, |model, cx| {
      model.set_panes_visible(true, cx);
      if let Some(id) = model.active_pane() {
        model.activate_pane(id, window, cx);
      }
    });
  }

  fn split_terminal(&self, axis: Axis, window: &mut Window, cx: &mut Context<Self>) {
    self.show_terminal_tab(cx);
    self.terminal.update(cx, |model, cx| {
      model.set_panes_visible(true, cx);
      if let Some(id) = model.active_pane() {
        model.split(id, axis, window, cx);
      }
    });
  }

  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  fn toggle_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let next = self.layout.update(cx, |layout, cx| {
      let next = if layout.layout().main_view == MainView::Settings {
        MainView::Changes
      } else {
        MainView::Settings
      };
      layout.select_main_view(next, cx);
      if next == MainView::Settings {
        layout.select_sidebar_view(SidebarView::Scm, cx);
      }
      next
    });
    if next == MainView::Settings {
      self.settings_restore = window.focused(cx).map(|handle| handle.downgrade());
      self.settings.update(cx, |settings, cx| {
        settings.focus(window, cx);
        settings.on_show(window, cx);
      });
    } else if let Some(handle) = self.settings_restore.take().and_then(|handle| handle.upgrade()) {
      handle.focus(window, cx);
    } else {
      self.focus(window, cx);
    }
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
      self.explorer().clone().into_any_element()
    };
    let sidebar = render_sidebar(layout.sidebar_view, select, sidebar_body, cx).into_any_element();
    let main_panel = render_main_panel(
      layout.main_view,
      &self.diff,
      &self.file,
      &self.history,
      &self.settings,
      cx,
    )
    .into_any_element();
    let terminal = self.terminal_panel.clone().into_any_element();
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
      .on_action(cx.listener(|this, _: &ShowSettings, window, cx| {
        this.toggle_settings(window, cx);
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
        if !window.has_focused_input(cx) && !this.explorer().read(cx).owns_focus(window, cx) {
          this.send(Intent::ClearFile, window, cx);
          if this.layout.read(cx).layout().main_view == MainView::File {
            this.model.update(cx, |model, cx| model.close_file(cx));
          }
        }
      }))
      .on_action(cx.listener(|this, _: &ToggleTerminal, window, cx| {
        let (visible, tab) = this.layout.update(cx, |layout, cx| {
          let visible = !layout.layout().terminal_visible;
          layout.set_terminal_visible(visible, cx);
          (visible, layout.layout().panel_tab)
        });
        this.terminal.update(cx, |model, cx| {
          model.set_panes_visible(visible && tab == PanelTab::Terminal, cx);
          if visible {
            model.ensure_group(window, cx);
          }
        });
      }))
      .on_action(cx.listener(|this, _: &FocusTerminal, window, cx| {
        this.show_terminal_tab(cx);
        this.terminal.update(cx, |model, cx| {
          model.set_panes_visible(true, cx);
          model.ensure_group(window, cx);
          if let Some(id) = model.active_pane() {
            model.activate_pane(id, window, cx);
          }
        });
      }))
      .on_action(cx.listener(|this, _: &NewTerminal, window, cx| {
        this.show_terminal_tab(cx);
        this.terminal.update(cx, |model, cx| {
          model.set_panes_visible(true, cx);
          model.new_group(window, cx);
        });
      }))
      .on_action(cx.listener(|this, _: &ShowOutputTab, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.set_panel_tab(PanelTab::GitOutput, cx));
        this.terminal.update(cx, |model, cx| model.set_panes_visible(false, cx));
      }))
      .on_action(cx.listener(|this, _: &ShowTerminalTab, window, cx| {
        this.show_terminal_tab(cx);
        this.terminal.update(cx, |model, cx| {
          model.set_panes_visible(true, cx);
          model.ensure_group(window, cx);
          if let Some(id) = model.active_pane() {
            model.activate_pane(id, window, cx);
          }
        });
      }))
      .on_action(cx.listener(|this, _: &ToggleTerminalMaximize, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.toggle_terminal_maximized(cx));
      }))
      .on_action(cx.listener(|this, _: &ClosePanel, _, cx| {
        this
          .layout
          .update(cx, |layout, cx| layout.set_terminal_visible(false, cx));
        this.terminal.update(cx, |model, cx| model.set_panes_visible(false, cx));
      }))
      .on_action(cx.listener(|this, _: &KillTerminal, window, cx| {
        this.terminal.update(cx, |model, cx| {
          if let Some(id) = model.active_group {
            model.kill_group(id, Some(window), cx);
          }
        });
      }))
      .on_action(cx.listener(|this, _: &KillTerminalPane, window, cx| {
        this.terminal.update(cx, |model, cx| {
          if let Some(id) = model.active_pane() {
            model.kill_pane(id, Some(window), cx);
          }
        });
      }))
      .on_action(cx.listener(|this, _: &SplitTerminalHorizontal, window, cx| {
        this.split_terminal(Axis::Vertical, window, cx);
      }))
      .on_action(cx.listener(|this, _: &SplitTerminalVertical, window, cx| {
        this.split_terminal(Axis::Horizontal, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup1, window, cx| {
        this.activate_terminal_group(1, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup2, window, cx| {
        this.activate_terminal_group(2, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup3, window, cx| {
        this.activate_terminal_group(3, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup4, window, cx| {
        this.activate_terminal_group(4, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup5, window, cx| {
        this.activate_terminal_group(5, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup6, window, cx| {
        this.activate_terminal_group(6, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup7, window, cx| {
        this.activate_terminal_group(7, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup8, window, cx| {
        this.activate_terminal_group(8, window, cx);
      }))
      .on_action(cx.listener(|this, _: &ActivateTerminalGroup9, window, cx| {
        this.activate_terminal_group(9, window, cx);
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
      .on_action(cx.listener(|this, _: &GitSync, window, cx| {
        dispatch_item(&this.model, OverflowItem::Sync, window, cx);
      }))
      .on_action(cx.listener(|this, _: &GitPullRebase, window, cx| {
        dispatch_item(&this.model, OverflowItem::PullRebase, window, cx);
      }))
      .on_action(cx.listener(|this, _: &GitPushForce, window, cx| {
        dispatch_item(&this.model, OverflowItem::PushForce, window, cx);
      }))
      .on_action(cx.listener(|this, _: &GitDiscardAll, window, cx| {
        dispatch_item(&this.model, OverflowItem::DiscardAll, window, cx);
      }))
      .on_action(cx.listener(|this, _: &GitStashIncludeUntracked, window, cx| {
        dispatch_item(&this.model, OverflowItem::StashIncludeUntracked, window, cx);
      }))
      .on_action(cx.listener(|this, _: &GitStashStagedOnly, window, cx| {
        dispatch_item(&this.model, OverflowItem::StashStagedOnly, window, cx);
      }))
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

  #[gpui_kit::test]
  fn git_sync_on_focused_repo_view_marks_sync_running(cx: &mut TestAppContext) {
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
        dispatch_item(&view.model, OverflowItem::Sync, window, cx);
        assert!(
          view.model().read(cx).state().running.contains(&NetworkOp::Sync),
          "GitSync body on focused RepoView should mark NetworkOp::Sync running"
        );
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn activate_terminal_group_out_of_range_does_not_show_panel(cx: &mut TestAppContext) {
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
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, false));
        let output = cx.new(|_| OutputLog::default());
        RepoView::new(model, layout, output, window, cx)
      }
    });
    window
      .update(cx, |view, window, cx| {
        view.focus(window, cx);
        view
          .layout()
          .update(cx, |layout, cx| layout.set_terminal_visible(false, cx));
        view.activate_terminal_group(9, window, cx);
        assert!(!view.layout().read(cx).layout().terminal_visible);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn split_actions_select_the_terminal_tab(cx: &mut TestAppContext) {
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
        view.layout().update(cx, |layout, cx| {
          layout.set_panel_tab(PanelTab::GitOutput, cx);
          layout.toggle_terminal_maximized(cx);
        });
        view.split_terminal(Axis::Vertical, window, cx);
        assert_eq!(view.layout().read(cx).layout().panel_tab, PanelTab::Terminal);
        view
          .layout()
          .update(cx, |layout, cx| layout.set_panel_tab(PanelTab::GitOutput, cx));
        view.split_terminal(Axis::Horizontal, window, cx);
        assert_eq!(view.layout().read(cx).layout().panel_tab, PanelTab::Terminal);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn explorer_to_settings_selects_changes(cx: &mut TestAppContext) {
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
        view.layout().update(cx, |layout, cx| {
          layout.select_sidebar_view(SidebarView::Explorer, cx);
        });
        assert_eq!(view.layout().read(cx).layout().sidebar_view, SidebarView::Explorer);
        assert_eq!(view.layout().read(cx).layout().main_view, MainView::File);
        view.settings.update(cx, |settings, _cx| {
          settings.stub_identity(String::new(), String::new());
        });
        view.toggle_settings(window, cx);
        assert_eq!(view.layout().read(cx).layout().sidebar_view, SidebarView::Scm);
        assert_eq!(view.layout().read(cx).layout().main_view, MainView::Settings);
      })
      .unwrap();
    cx.run_until_parked();
  }

  #[gpui_kit::test]
  fn closing_settings_restores_the_opener_focus(cx: &mut TestAppContext) {
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
        view.settings.update(cx, |settings, _cx| {
          settings.stub_identity(String::new(), String::new());
        });
        let commit = view.changes().read(cx).commit.clone();
        commit.update(cx, |state, cx| state.focus(window, cx));
        let commit_handle = commit.read(cx).focus_handle(cx).clone();
        assert_eq!(window.focused(cx).as_ref(), Some(&commit_handle));
        view.toggle_settings(window, cx);
        assert_eq!(view.layout().read(cx).layout().main_view, MainView::Settings);
        assert_ne!(window.focused(cx).as_ref(), Some(&commit_handle));
        view.toggle_settings(window, cx);
        assert_eq!(view.layout().read(cx).layout().main_view, MainView::Changes);
        assert_eq!(window.focused(cx).as_ref(), Some(&commit_handle));
      })
      .unwrap();
    cx.run_until_parked();
  }
}
