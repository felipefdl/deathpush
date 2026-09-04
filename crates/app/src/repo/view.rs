#![allow(dead_code)]

use deathpush_core::config::layout::{MainView, PanelTab, SidebarView};
use deathpush_core::config::settings::{DiffLayout, SidebarPosition};
use deathpush_core::session::types::Intent;
use gpui_kit::base::{ResizableState, h_resizable, resizable_panel, v_resizable};
use gpui_kit::component::WindowExt;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::layout_model::LayoutModel;
use super::main_panel::render_main_panel;
use super::model::RepoModel;
use super::output_log::OutputLog;
use super::sidebar::render_sidebar;
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
  body_state: Entity<ResizableState>,
  main_state: Entity<ResizableState>,
  focus_handle: FocusHandle,
}

impl RepoView {
  pub fn new(
    model: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    output: Entity<OutputLog>,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&model, |_, _, cx| cx.notify()).detach();
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    cx.observe(&output, |_, _, cx| cx.notify()).detach();
    Self {
      model,
      layout,
      output,
      body_state: cx.new(|_| ResizableState::default()),
      main_state: cx.new(|_| ResizableState::default()),
      focus_handle: cx.focus_handle(),
    }
  }

  pub fn model(&self) -> &Entity<RepoModel> {
    &self.model
  }

  pub fn layout(&self) -> &Entity<LayoutModel> {
    &self.layout
  }

  pub fn output(&self) -> &Entity<OutputLog> {
    &self.output
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
    let sidebar =
      render_sidebar(layout.sidebar_view, select, div().size_full().into_any_element(), cx).into_any_element();
    let main_panel = render_main_panel(layout.main_view, cx).into_any_element();
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
    let state = self.model.read(cx).state().clone();
    let status_bar = render_status_bar(&state, window, cx).into_any_element();
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
      .on_action(cx.listener(|this, _: &GitPull, window, cx| this.send(Intent::Pull { rebase: false }, window, cx)))
      .on_action(cx.listener(|this, _: &GitPush, window, cx| {
        this.send(
          Intent::Push {
            force: false,
            confirmed: false,
          },
          window,
          cx,
        )
      }))
      .on_action(cx.listener(|this, _: &GitFetch, window, cx| this.send(Intent::Fetch { prune: true }, window, cx)))
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
