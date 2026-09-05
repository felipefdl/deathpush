use std::sync::Arc;

use deathpush_core::Core;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use super::git_identity::{GitIdentity, IDENTITY_DEBOUNCE_MS, should_save};
use super::sections;
use crate::config::AppConfig;
use crate::repo::changes::filter::debounce;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::{ActivePalette, ThemeCatalog, hsla};

pub struct SettingsView {
  #[allow(dead_code)]
  repo: Entity<RepoModel>,
  #[allow(dead_code)]
  layout: Entity<LayoutModel>,
  identity: GitIdentity,
  name_input: Entity<InputState>,
  email_input: Entity<InputState>,
  ui_font_input: Entity<InputState>,
  editor_font_input: Entity<InputState>,
  focus_handle: FocusHandle,
  core: Arc<Core>,
}

impl SettingsView {
  pub fn new(
    repo: Entity<RepoModel>,
    layout: Entity<LayoutModel>,
    core: Arc<Core>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    cx.observe(&layout, |_, _, cx| cx.notify()).detach();
    let ui_font = AppConfig::get(cx).settings.ui.font_family.clone();
    let editor_font = AppConfig::get(cx).settings.editor.font_family.clone();
    let ui_font_input = cx.new(|cx| InputState::new(window, cx).default_value(ui_font));
    let editor_font_input = cx.new(|cx| InputState::new(window, cx).default_value(editor_font));
    let name_input = cx.new(|cx| InputState::new(window, cx));
    let email_input = cx.new(|cx| InputState::new(window, cx));

    cx.subscribe(&ui_font_input, |_, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let value = input.read(cx).value().to_string();
        AppConfig::update(cx, |c| c.settings.ui.font_family = value);
        cx.notify();
      }
    })
    .detach();
    cx.subscribe(&editor_font_input, |_, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let value = input.read(cx).value().to_string();
        AppConfig::update(cx, |c| c.settings.editor.font_family = value);
        cx.notify();
      }
    })
    .detach();
    cx.subscribe(&name_input, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.identity.name_gen + 1;
        debounce(
          cx,
          &mut this.identity.name_gen,
          IDENTITY_DEBOUNCE_MS,
          move |this, cx| {
            if this.identity.name_gen != token {
              return;
            }
            let current = this.name_input.read(cx).value().to_string();
            if should_save(&this.identity.name, &current) {
              this.save_git_config("user.name", current, true, cx);
            }
          },
        );
      }
    })
    .detach();
    cx.subscribe(&email_input, |this, _, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let token = this.identity.email_gen + 1;
        debounce(
          cx,
          &mut this.identity.email_gen,
          IDENTITY_DEBOUNCE_MS,
          move |this, cx| {
            if this.identity.email_gen != token {
              return;
            }
            let current = this.email_input.read(cx).value().to_string();
            if should_save(&this.identity.email, &current) {
              this.save_git_config("user.email", current, false, cx);
            }
          },
        );
      }
    })
    .detach();

    let view = Self {
      repo,
      layout,
      identity: GitIdentity::new(),
      name_input,
      email_input,
      ui_font_input,
      editor_font_input,
      focus_handle: cx.focus_handle(),
      core: core.clone(),
    };
    view.load_identity(window, cx);
    view
  }

  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  fn load_identity(&self, window: &mut Window, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let task = core.clone().spawn(async move {
      let name = core.get_git_config("user.name").await.unwrap_or_default();
      let email = core.get_git_config("user.email").await.unwrap_or_default();
      (name, email)
    });
    cx.spawn_in(window, async move |this, cx| {
      let Ok((name, email)) = task.await else {
        return;
      };
      let _ = this.update_in(cx, |this, window, cx| {
        if this.identity.name_gen == 0 {
          this.identity.name = name.clone();
          this
            .name_input
            .update(cx, |state, cx| state.set_value(name, window, cx));
        }
        if this.identity.email_gen == 0 {
          this.identity.email = email.clone();
          this
            .email_input
            .update(cx, |state, cx| state.set_value(email, window, cx));
        }
      });
    })
    .detach();
  }

  fn save_git_config(&mut self, key: &'static str, value: String, is_name: bool, cx: &mut Context<Self>) {
    let core = self.core.clone();
    let saved = value.clone();
    let task = core
      .clone()
      .spawn(async move { core.set_git_config(key, &value).await });
    cx.spawn(async move |this, cx| match task.await {
      Ok(Ok(())) => {
        let _ = this.update(cx, |this, _| {
          if is_name {
            this.identity.name = saved;
          } else {
            this.identity.email = saved;
          }
        });
      }
      Ok(Err(err)) => tracing::warn!("git config {key}: {err}"),
      Err(err) => tracing::warn!("git config {key}: {err}"),
    })
    .detach();
  }
}

impl Render for SettingsView {
  fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    let palette = cx.global::<ActivePalette>().0;
    let settings = AppConfig::get(cx).settings.clone();
    let catalog = ThemeCatalog::get(cx).entries.clone();
    let view = cx.weak_entity();
    div()
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .flex_col()
      .text_size(px(13.0))
      .child(
        div()
          .h(px(35.0))
          .flex_shrink_0()
          .flex()
          .items_center()
          .justify_between()
          .px_3()
          .border_b_1()
          .border_color(hsla(palette.border))
          .child(
            div()
              .text_size(px(14.0))
              .font_weight(FontWeight::SEMIBOLD)
              .child("Settings"),
          )
          .child(
            Button::new("reset-to-defaults")
              .outline()
              .small()
              .label("Reset to Defaults"),
          ),
      )
      .child(
        div()
          .id("settings-body")
          .flex_1()
          .min_h_0()
          .overflow_y_scroll()
          .px_4()
          .py_3()
          .flex()
          .flex_col()
          .gap_5()
          .child(sections::appearance(
            &settings,
            &catalog,
            &self.ui_font_input,
            view.clone(),
            cx,
          ))
          .child(sections::editor(
            &settings.editor,
            &self.editor_font_input,
            view.clone(),
            cx,
          ))
          .child(sections::diff_viewer(&settings.diff, view.clone(), cx))
          .child(sections::git(
            &settings.git,
            &self.name_input,
            &self.email_input,
            view,
            cx,
          ))
          .child(sections::projects(&settings, cx)),
      )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  use deathpush_core::config::settings::WorkspaceEntry;
  use deathpush_core::session::types::{
    OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{RepoOperationState, StatusPhase};
  use gpui_kit::TestAppContext;

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
  fn settings_view_renders(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      AppConfig::update(cx, |c| {
        c.settings.editor.font_family = "Test Mono".into();
        c.settings.projects.workspaces = vec![WorkspaceEntry {
          directory: "/src".into(),
          scan_depth: 2,
        }];
      });
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
        SettingsView::new(model, layout, core, window, cx)
      }
    });
    window
      .update(cx, |view, window, cx| {
        window.refresh();
        assert_eq!(view.editor_font_input.read(cx).value().as_ref(), "Test Mono");
        assert_eq!(
          deathpush_core::config::settings_ui::workspace_summary(&AppConfig::get(cx).settings.projects.workspaces)
            .as_deref(),
          Some("/src:2")
        );
      })
      .unwrap();
  }
}
