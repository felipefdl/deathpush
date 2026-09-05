use std::sync::Arc;

#[cfg(test)]
use std::time::Duration;

use deathpush_core::Core;
use deathpush_core::config::settings::Settings;
use deathpush_core::config::settings_ui::{RESET_MESSAGE, RESET_TITLE, ShellPreset, preset_for, shell_presets};
use gpui_kit::component::Sizable;
use gpui_kit::component::button::Button;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::*;

use super::git_identity::{GitIdentity, IDENTITY_DEBOUNCE_MS, should_apply_loaded, should_save};
use super::sections;
use crate::config::AppConfig;
use crate::repo::changes::filter::debounce;
use crate::repo::layout_model::LayoutModel;
use crate::repo::model::RepoModel;
use crate::theme::{ActivePalette, ThemeCatalog, hsla};

/// Replace app settings with defaults. Git identity is not an app setting.
pub(crate) fn reset_decision(_settings: Settings, name: String, email: String) -> (Settings, String, String) {
  (Settings::default(), name, email)
}

/// App settings in the repository main panel.
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
  terminal_font_input: Entity<InputState>,
  shell_path_input: Entity<InputState>,
  pub(crate) shell_custom: bool,
  focus_handle: FocusHandle,
  core: Arc<Core>,
  #[cfg(test)]
  identity_load: Option<Arc<std::sync::Mutex<(String, String)>>>,
  #[cfg(test)]
  save_delay: Duration,
  #[cfg(test)]
  saves: Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl SettingsView {
  /// Build the page. Git identity is loaded when the page is shown.
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
    let terminal_font = AppConfig::get(cx).settings.terminal.font_family.clone();
    let shell_path = AppConfig::get(cx).settings.terminal.shell_path.clone();
    let presets = shell_presets(&|path| sections::env_shell_exists(path));
    let shell_custom = matches!(preset_for(&shell_path, &presets), ShellPreset::Custom);
    let custom_value = if shell_custom { shell_path } else { String::new() };
    let ui_font_input = cx.new(|cx| InputState::new(window, cx).default_value(ui_font));
    let editor_font_input = cx.new(|cx| InputState::new(window, cx).default_value(editor_font));
    let terminal_font_input = cx.new(|cx| InputState::new(window, cx).default_value(terminal_font));
    let shell_path_input = cx.new(|cx| InputState::new(window, cx).default_value(custom_value));
    let name_input = cx.new(|cx| InputState::new(window, cx));
    let email_input = cx.new(|cx| InputState::new(window, cx));

    cx.subscribe(&ui_font_input, |_, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let value = input.read(cx).value().to_string();
        AppConfig::update(cx, |c| c.settings.ui.font_family = value);
        crate::theme::refresh_ui_font(None, cx);
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
    cx.subscribe(&terminal_font_input, |_, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) {
        let value = input.read(cx).value().to_string();
        AppConfig::update(cx, |c| c.settings.terminal.font_family = value);
        cx.notify();
      }
    })
    .detach();
    cx.subscribe(&shell_path_input, |this, input, event: &InputEvent, cx| {
      if matches!(event, InputEvent::Change) && this.shell_custom {
        let value = input.read(cx).value().to_string();
        AppConfig::update(cx, |c| c.settings.terminal.shell_path = value);
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
            this.identity.name_ready_gen = token;
            this.maybe_save_name(cx);
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
            this.identity.email_ready_gen = token;
            this.maybe_save_email(cx);
          },
        );
      }
    })
    .detach();

    Self {
      repo,
      layout,
      identity: GitIdentity::new(),
      name_input,
      email_input,
      ui_font_input,
      editor_font_input,
      terminal_font_input,
      shell_path_input,
      shell_custom,
      focus_handle: cx.focus_handle(),
      core: core.clone(),
      #[cfg(test)]
      identity_load: None,
      #[cfg(test)]
      save_delay: Duration::ZERO,
      #[cfg(test)]
      saves: Arc::new(std::sync::Mutex::new(Vec::new())),
    }
  }

  /// Move focus to the settings page.
  pub fn focus(&self, window: &mut Window, cx: &mut App) {
    self.focus_handle.focus(window, cx);
  }

  /// Reload Git identity from config. Skips a field that is focused or has a save in flight.
  pub fn on_show(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.load_identity(window, cx);
  }

  fn confirm_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let answer = window.prompt(
      PromptLevel::Warning,
      RESET_TITLE,
      Some(RESET_MESSAGE),
      &["Reset", "Cancel"],
      cx,
    );
    cx.spawn_in(window, async move |this, cx| {
      if !matches!(answer.await, Ok(0)) {
        return;
      }
      let _ = this.update_in(cx, |this, window, cx| this.apply_reset(window, cx));
    })
    .detach();
  }

  fn apply_reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let current = AppConfig::get(cx).settings.clone();
    let (settings, _, _) = reset_decision(current, self.identity.name.clone(), self.identity.email.clone());
    AppConfig::update(cx, |c| c.settings = settings);
    crate::theme::apply_for_appearance(cx.window_appearance(), Some(window), cx);
    crate::zoom::set_zoom_level(0, cx);
    self.shell_custom = false;
    let defaults = Settings::default();
    self.ui_font_input.update(cx, |state, cx| {
      state.set_value(defaults.ui.font_family.clone(), window, cx);
    });
    self.editor_font_input.update(cx, |state, cx| {
      state.set_value(defaults.editor.font_family.clone(), window, cx);
    });
    self.terminal_font_input.update(cx, |state, cx| {
      state.set_value(defaults.terminal.font_family.clone(), window, cx);
    });
    self.shell_path_input.update(cx, |state, cx| {
      state.set_value(defaults.terminal.shell_path.clone(), window, cx);
    });
    cx.notify();
  }

  fn load_identity(&self, window: &mut Window, cx: &mut Context<Self>) {
    #[cfg(test)]
    if let Some(load) = self.identity_load.clone() {
      cx.spawn_in(window, async move |this, cx| {
        let (name, email) = load.lock().expect("identity load").clone();
        let _ = this.update_in(cx, |this, window, cx| {
          this.apply_identity_values(name, email, window, cx);
        });
      })
      .detach();
      return;
    }
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
        this.apply_identity_values(name, email, window, cx);
      });
    })
    .detach();
  }

  pub(crate) fn apply_identity_values(
    &mut self,
    name: String,
    email: String,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let name_focused = self.name_input.read(cx).focus_handle(cx).is_focused(window);
    if should_apply_loaded(self.identity.name_pending(), name_focused) {
      self.identity.name = name.clone();
      self
        .name_input
        .update(cx, |state, cx| state.set_value(name, window, cx));
      self.identity.name_done_gen = self.identity.name_gen;
    }
    let email_focused = self.email_input.read(cx).focus_handle(cx).is_focused(window);
    if should_apply_loaded(self.identity.email_pending(), email_focused) {
      self.identity.email = email.clone();
      self
        .email_input
        .update(cx, |state, cx| state.set_value(email, window, cx));
      self.identity.email_done_gen = self.identity.email_gen;
    }
  }

  fn save_git_config(&mut self, key: &'static str, value: String, is_name: bool, token: u64, cx: &mut Context<Self>) {
    if is_name {
      self.identity.name_inflight = Some(token);
    } else {
      self.identity.email_inflight = Some(token);
    }
    #[cfg(test)]
    if self.identity_load.is_some() {
      let delay = self.save_delay;
      let saved = value.clone();
      let recorded_key = key.to_string();
      let saves = self.saves.clone();
      cx.spawn(async move |this, cx| {
        if !delay.is_zero() {
          cx.background_executor().timer(delay).await;
        }
        saves
          .lock()
          .expect("identity saves")
          .push((recorded_key, saved.clone()));
        let _ = this.update(cx, |this, cx| {
          this.complete_identity_save(is_name, token, saved, true, cx);
        });
      })
      .detach();
      return;
    }
    let core = self.core.clone();
    let saved = value.clone();
    let task = core
      .clone()
      .spawn(async move { core.set_git_config(key, &value).await });
    cx.spawn(async move |this, cx| {
      let result = task.await;
      let ok = matches!(&result, Ok(Ok(())));
      match result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => tracing::warn!("git config {key}: {err}"),
        Err(err) => tracing::warn!("git config {key}: {err}"),
      }
      let _ = this.update(cx, |this, cx| {
        this.complete_identity_save(is_name, token, saved, ok, cx);
      });
    })
    .detach();
  }

  fn maybe_save_name(&mut self, cx: &mut Context<Self>) {
    let current = self.name_input.read(cx).value().to_string();
    if should_save(&self.identity.name, &current) {
      if self.identity.name_inflight.is_none() {
        self.save_git_config("user.name", current, true, self.identity.name_gen, cx);
      }
    } else if self.identity.name_inflight.is_none() {
      self.identity.name_done_gen = self.identity.name_gen;
    }
  }

  fn maybe_save_email(&mut self, cx: &mut Context<Self>) {
    let current = self.email_input.read(cx).value().to_string();
    if should_save(&self.identity.email, &current) {
      if self.identity.email_inflight.is_none() {
        self.save_git_config("user.email", current, false, self.identity.email_gen, cx);
      }
    } else if self.identity.email_inflight.is_none() {
      self.identity.email_done_gen = self.identity.email_gen;
    }
  }

  fn complete_identity_save(&mut self, is_name: bool, token: u64, saved: String, ok: bool, cx: &mut Context<Self>) {
    if is_name {
      if self.identity.name_inflight == Some(token) {
        self.identity.name_inflight = None;
      }
      if ok {
        self.identity.name = saved;
      }
      if self.identity.name_gen != token {
        if self.identity.name_ready_gen == self.identity.name_gen {
          self.maybe_save_name(cx);
        } else if self.identity.name_inflight.is_none() {
          let current = self.name_input.read(cx).value().to_string();
          if !should_save(&self.identity.name, &current) {
            self.identity.name_done_gen = self.identity.name_gen;
          }
        }
        return;
      }
      self.identity.name_done_gen = token;
      return;
    }
    if self.identity.email_inflight == Some(token) {
      self.identity.email_inflight = None;
    }
    if ok {
      self.identity.email = saved;
    }
    if self.identity.email_gen != token {
      if self.identity.email_ready_gen == self.identity.email_gen {
        self.maybe_save_email(cx);
      } else if self.identity.email_inflight.is_none() {
        let current = self.email_input.read(cx).value().to_string();
        if !should_save(&self.identity.email, &current) {
          self.identity.email_done_gen = self.identity.email_gen;
        }
      }
      return;
    }
    self.identity.email_done_gen = token;
  }

  #[cfg(test)]
  pub(crate) fn stub_identity(&mut self, name: String, email: String) {
    self.identity_load = Some(Arc::new(std::sync::Mutex::new((name, email))));
  }

  #[cfg(test)]
  pub(crate) fn set_stub_identity(&mut self, name: String, email: String) {
    if let Some(load) = &self.identity_load {
      *load.lock().expect("identity load") = (name, email);
    } else {
      self.stub_identity(name, email);
    }
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
              .label("Reset to Defaults")
              .on_click(cx.listener(|this, _, window, cx| this.confirm_reset(window, cx))),
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
            view.clone(),
            cx,
          ))
          .child(sections::projects(&settings, cx))
          .child(sections::terminal(
            &settings.terminal,
            &self.terminal_font_input,
            &self.shell_path_input,
            self.shell_custom,
            view,
            cx,
          )),
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
  use gpui_kit::{TestAppContext, WindowHandle};

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

  #[test]
  fn reset_keeps_git_identity() {
    let mut settings = Settings::default();
    settings.git.blame = false;
    settings.ui.font_size = 20;
    settings.terminal.scrollback = 1000;
    let (reset, name, email) = reset_decision(settings, "Ada".into(), "ada@example.com".into());
    assert_eq!(reset, Settings::default());
    assert!(reset.git.blame);
    assert_eq!(name, "Ada");
    assert_eq!(email, "ada@example.com");
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
        c.settings.terminal.font_family = "Term Mono".into();
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
        assert_eq!(view.terminal_font_input.read(cx).value().as_ref(), "Term Mono");
        assert_eq!(
          deathpush_core::config::settings_ui::workspace_summary(&AppConfig::get(cx).settings.projects.workspaces)
            .as_deref(),
          Some("/src:2")
        );
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn reset_replaces_settings_without_touching_identity_fields(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let window = open_settings(cx, &config_dir, &resource_dir);
    window
      .update(cx, |view, window, cx| {
        view.apply_identity_values("Ada".into(), "ada@example.com".into(), window, cx);
        AppConfig::update(cx, |c| {
          c.settings.ui.font_size = 20;
          c.settings.git.blame = false;
          c.settings.terminal.scrollback = 1000;
          c.settings.editor.font_family = "Test Mono".into();
        });
        view.apply_reset(window, cx);
        let settings = AppConfig::get(cx).settings.clone();
        assert_eq!(settings.ui.font_size, Settings::default().ui.font_size);
        assert_eq!(settings.git, Settings::default().git);
        assert_eq!(settings.terminal.scrollback, Settings::default().terminal.scrollback);
        assert_eq!(settings.editor.font_family, Settings::default().editor.font_family);
        assert_eq!(view.identity.name, "Ada");
        assert_eq!(view.identity.email, "ada@example.com");
        assert_eq!(view.name_input.read(cx).value().as_ref(), "Ada");
        assert_eq!(view.email_input.read(cx).value().as_ref(), "ada@example.com");
        assert_eq!(
          view.editor_font_input.read(cx).value().as_ref(),
          Settings::default().editor.font_family
        );
      })
      .unwrap();
  }

  fn open_settings(
    cx: &mut TestAppContext,
    config_dir: &tempfile::TempDir,
    resource_dir: &tempfile::TempDir,
  ) -> WindowHandle<SettingsView> {
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
      let snapshot = snapshot(&root);
      let layout_dir = layout_dir.clone();
      let root = root.clone();
      move |window, cx| {
        let model = cx.new(|_| RepoModel::new(core.clone(), session, snapshot));
        let layout = cx.new(|_| LayoutModel::load_from(layout_dir, &root, true));
        SettingsView::new(model, layout, core, window, cx)
      }
    })
  }

  #[gpui_kit::test]
  fn identity_reapplies_on_show_unless_editing(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let window = open_settings(cx, &config_dir, &resource_dir);
    window
      .update(cx, |view, window, cx| {
        view.stub_identity("Ada".into(), "ada@x".into());
        view.on_show(window, cx);
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.name_input.read(cx).value().as_ref(), "Ada");
        assert_eq!(view.email_input.read(cx).value().as_ref(), "ada@x");
        view.set_stub_identity("Grace".into(), "grace@x".into());
        view.on_show(window, cx);
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.name_input.read(cx).value().as_ref(), "Grace");
        assert_eq!(view.email_input.read(cx).value().as_ref(), "grace@x");
        view.name_input.update(cx, |state, cx| state.focus(window, cx));
        view.set_stub_identity("Other".into(), "other@x".into());
        view.on_show(window, cx);
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.name_input.read(cx).value().as_ref(), "Grace");
        assert_eq!(view.email_input.read(cx).value().as_ref(), "other@x");
        view.focus(window, cx);
        view.identity.name_gen += 1;
        view.set_stub_identity("Pending".into(), "p@x".into());
        view.on_show(window, cx);
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |view, _, cx| {
        assert_eq!(view.name_input.read(cx).value().as_ref(), "Grace");
        assert_eq!(view.email_input.read(cx).value().as_ref(), "p@x");
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn identity_save_does_not_reset_a_newer_generation(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let window = open_settings(cx, &config_dir, &resource_dir);
    window
      .update(cx, |view, window, cx| {
        view.stub_identity(String::new(), String::new());
        view.save_delay = Duration::from_millis(2000);
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Ada", window, cx));
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(IDENTITY_DEBOUNCE_MS));
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.identity.name_inflight, Some(1));
        assert_eq!(view.identity.name_gen, 1);
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Grace", window, cx));
      })
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name_gen, 2);
        assert_eq!(view.identity.name_inflight, Some(1));
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(2000));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name_gen, 2);
        assert_eq!(view.identity.name_inflight, Some(2));
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Ada".into())]
        );
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(2000));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name, "Grace");
        assert_eq!(view.identity.name_gen, 2);
        assert_eq!(view.identity.name_done_gen, 2);
        assert!(view.identity.name_inflight.is_none());
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Ada".into()), ("user.name".into(), "Grace".into()),]
        );
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn identity_reversion_after_inflight_save_writes_the_baseline(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let window = open_settings(cx, &config_dir, &resource_dir);
    window
      .update(cx, |view, window, cx| {
        view.stub_identity(String::new(), String::new());
        view.save_delay = Duration::from_millis(2000);
        view.apply_identity_values("Ada".into(), String::new(), window, cx);
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Grace", window, cx));
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(IDENTITY_DEBOUNCE_MS));
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.identity.name_inflight, Some(1));
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Ada", window, cx));
      })
      .unwrap();
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(2000));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name_gen, 2);
        assert_eq!(view.identity.name_inflight, Some(2));
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Grace".into())]
        );
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(2000));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name, "Ada");
        assert_eq!(view.identity.name_done_gen, 2);
        assert!(view.identity.name_inflight.is_none());
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Grace".into()), ("user.name".into(), "Ada".into()),]
        );
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn identity_coalesced_save_waits_for_debounce(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let window = open_settings(cx, &config_dir, &resource_dir);
    window
      .update(cx, |view, window, cx| {
        view.stub_identity(String::new(), String::new());
        view.save_delay = Duration::from_millis(100);
        view.apply_identity_values("Ada".into(), String::new(), window, cx);
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Grace", window, cx));
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(IDENTITY_DEBOUNCE_MS));
    cx.run_until_parked();
    window
      .update(cx, |view, window, cx| {
        assert_eq!(view.identity.name_inflight, Some(1));
        view
          .name_input
          .update(cx, |state, cx| state.replace_all("Ada", window, cx));
      })
      .unwrap();
    cx.run_until_parked();
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name, "Grace");
        assert_eq!(view.identity.name_gen, 2);
        assert!(view.identity.name_inflight.is_none());
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Grace".into())]
        );
      })
      .unwrap();
    cx.executor()
      .advance_clock(Duration::from_millis(IDENTITY_DEBOUNCE_MS - 100));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name_inflight, Some(2));
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Grace".into())]
        );
      })
      .unwrap();
    cx.executor().advance_clock(Duration::from_millis(100));
    cx.run_until_parked();
    window
      .update(cx, |view, _, _| {
        assert_eq!(view.identity.name, "Ada");
        assert!(view.identity.name_inflight.is_none());
        assert_eq!(
          view.saves.lock().expect("identity saves").clone(),
          vec![("user.name".into(), "Grace".into()), ("user.name".into(), "Ada".into()),]
        );
      })
      .unwrap();
  }
}
