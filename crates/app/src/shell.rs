use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::config::windows::SavedWindow;
use deathpush_core::session::types::{Intent, IntentOutcome};
use deathpush_core::{Core, CoreEvent, SessionId};
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::actions::*;
use crate::config::AppConfig;
use crate::keymap::{CONTEXT_APP, CONTEXT_REPOSITORY, CONTEXT_WELCOME};
use crate::menus::{MenuContext, set_menu_context};
use crate::repo_placeholder::RepoPlaceholder;
use crate::theme::{ActivePalette, apply_for_appearance, hsla};
use crate::title_bar::render_title_bar;
use crate::window::open_shell_window;
use crate::zoom;

pub enum Screen {
  Boot,
  Welcome(Entity<crate::welcome::WelcomeView>),
  Repository(Entity<RepoPlaceholder>),
}

/// Overlays owned by the shell. Their views come in Task 9; the enum is the contract.
pub enum Overlay {
  Opening,
  // Task 9
  // Clone(Entity<crate::overlays::clone_dialog::CloneDialog>),
  // WorkspaceSettings(Entity<crate::overlays::workspace_settings::WorkspaceSettingsDialog>),
  // Licenses(Entity<crate::overlays::licenses::LicensesDialog>),
}

pub struct Shell {
  pub core: Arc<Core>,
  pub session: SessionId,
  pub screen: Screen,
  pub overlay: Option<Overlay>,
  pub toast: Option<SharedString>,
  pub title: SharedString,
  pub window_index: usize,
  focus_handle: FocusHandle,
  last_saved_bounds: Option<SavedWindow>,
  cli_installed: bool,
  opening_generation: u64,
}

impl Shell {
  pub fn new(
    core: Arc<Core>,
    window_index: usize,
    initial: Option<PathBuf>,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) -> Self {
    let (session, events) = core.open_session();
    let cli_installed = core
      .check_cli_installed()
      .map(|status| status.installed)
      .unwrap_or(false);
    let shell = Self {
      core,
      session,
      screen: Screen::Boot,
      overlay: None,
      toast: None,
      title: "DeathPush".into(),
      window_index,
      focus_handle: cx.focus_handle(),
      last_saved_bounds: None,
      cli_installed,
      opening_generation: 0,
    };
    shell.listen(events, cx);
    zoom::apply_zoom_to_window(zoom::current_level(cx), window);
    window
      .observe_window_appearance(|window, cx| {
        let appearance = window.appearance();
        apply_for_appearance(appearance, Some(window), cx);
      })
      .detach();
    cx.observe_window_activation(window, |this, window, cx| {
      this.sync_menus(window, cx);
    })
    .detach();
    match initial {
      Some(path) => cx.defer_in(window, move |this, window, cx| this.open_repository(path, window, cx)),
      None => cx.defer_in(window, |this, window, cx| this.show_welcome(window, cx)),
    }
    shell.focus_handle.focus(window, cx);
    shell
  }

  fn listen(&self, mut events: UnboundedReceiver<CoreEvent>, cx: &mut Context<Self>) {
    cx.spawn(async move |this, cx| {
      while let Some(event) = events.recv().await {
        let alive = this.update(cx, |this, cx| {
          if let (CoreEvent::SessionStatus(_), Screen::Repository(view)) = (&event, &this.screen) {
            view.update(cx, |view, cx| {
              view.status_events += 1;
              cx.notify();
            });
          }
          if let CoreEvent::WatcherError(message) = &event {
            this.show_toast(message.clone(), cx);
          }
        });
        if alive.is_err() {
          break;
        }
      }
    })
    .detach();
  }

  pub fn show_welcome(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let view = cx.new(|cx| crate::welcome::WelcomeView::new(window, cx));
    cx.subscribe_in(
      &view,
      window,
      |this, _, event: &crate::welcome::WelcomeEvent, window, cx| match event {
        crate::welcome::WelcomeEvent::Open(path) => this.open_repository(path.clone(), window, cx),
        crate::welcome::WelcomeEvent::Clone => this.open_clone_dialog(window, cx),
        crate::welcome::WelcomeEvent::ConfigureWorkspace => this.open_workspace_settings(window, cx),
      },
    )
    .detach();
    self.screen = Screen::Welcome(view);
    self.title = "DeathPush".into();
    window.set_window_title("DeathPush");
    self.sync_menus(window, cx);
    cx.notify();
  }

  #[allow(dead_code)]
  pub fn rescan_welcome(&self, cx: &mut Context<Self>) {
    if let Screen::Welcome(view) = &self.screen {
      view.update(cx, |view, cx| view.rescan(cx));
    }
  }

  pub fn open_repository(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
    if let Screen::Welcome(_) = &self.screen {
      self.overlay = Some(Overlay::Opening);
      self.opening_generation += 1;
      cx.notify();
    }
    let generation = self.opening_generation;
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let path_string = path.to_string_lossy().into_owned();
    let task = runtime.spawn(async move {
      core
        .session_intent(session, Intent::OpenRepository { path: path_string })
        .await
    });
    cx.spawn_in(window, async move |this, cx| {
      let result = task.await;
      let _ = this.update_in(cx, |this, window, cx| {
        if matches!(this.overlay, Some(Overlay::Opening)) && this.opening_generation == generation {
          this.overlay = None;
        }
        match result {
          Ok(Ok(IntentOutcome::Snapshot { snapshot })) => {
            let title = deathpush_core::ops::window_title(&snapshot.repo.root, snapshot.repo.head_branch.as_deref());
            let now = chrono::Utc::now().to_rfc3339();
            let root = snapshot.repo.root.clone();
            let branch = snapshot.repo.head_branch.clone();
            AppConfig::update(cx, move |config| config.recents.add(&root, branch, &now));
            this.title = title.clone().into();
            window.set_window_title(&title);
            let view = cx.new(|_| RepoPlaceholder {
              title: title.into(),
              status_events: 0,
            });
            this.screen = Screen::Repository(view);
            this.sync_menus(window, cx);
          }
          Ok(Ok(other)) => this.show_toast(format!("Unexpected outcome: {other:?}"), cx),
          Ok(Err(err)) => {
            this.show_toast(err.to_string(), cx);
            if matches!(this.screen, Screen::Boot) {
              this.show_welcome(window, cx);
            }
          }
          Err(err) => this.show_toast(err.to_string(), cx),
        }
        cx.notify();
      });
    })
    .detach();
  }

  pub fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.toast = Some(message.into());
    cx.notify();
  }

  #[allow(dead_code)]
  pub fn set_overlay(&mut self, overlay: Option<Overlay>, cx: &mut Context<Self>) {
    self.overlay = overlay;
    cx.notify();
  }

  #[allow(dead_code)]
  pub fn set_cli_installed(&mut self, installed: bool, window: &mut Window, cx: &mut Context<Self>) {
    self.cli_installed = installed;
    self.sync_menus(window, cx);
  }

  fn menu_context(&self) -> MenuContext {
    MenuContext {
      repo_open: matches!(self.screen, Screen::Repository(_)),
      cli_installed: self.cli_installed,
    }
  }

  fn sync_menus(&self, window: &Window, cx: &mut App) {
    if window.is_window_active() {
      set_menu_context(self.menu_context(), cx);
    }
  }

  fn save_bounds_if_changed(&mut self, window: &Window, cx: &mut App) {
    let bounds = window.bounds();
    let saved = SavedWindow {
      x: f32::from(bounds.origin.x),
      y: f32::from(bounds.origin.y),
      width: f32::from(bounds.size.width),
      height: f32::from(bounds.size.height),
      maximized: window.is_maximized(),
    };
    if self.last_saved_bounds != Some(saved) {
      self.last_saved_bounds = Some(saved);
      let index = self.window_index;
      AppConfig::update(cx, move |config| config.windows.record(index, saved));
    }
  }

  fn prompt_open_repository(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    let receiver = cx.prompt_for_paths(PathPromptOptions {
      files: false,
      directories: true,
      multiple: false,
      prompt: Some("Open Git Repository".into()),
    });
    cx.spawn_in(window, async move |this, cx| {
      if let Ok(Ok(Some(paths))) = receiver.await
        && let Some(path) = paths.into_iter().next()
      {
        let _ = this.update_in(cx, |this, window, cx| this.open_repository(path, window, cx));
      }
    })
    .detach();
  }

  fn open_clone_dialog(&mut self, _: &mut Window, cx: &mut Context<Self>) {
    self.show_toast("Coming in Task 9", cx);
  }

  fn open_licenses(&mut self, _: &mut Window, cx: &mut Context<Self>) {
    self.show_toast("Coming in Task 9", cx);
  }

  fn open_workspace_settings(&mut self, _: &mut Window, cx: &mut Context<Self>) {
    self.show_toast("Coming in Task 9", cx);
  }

  fn render_boot(&self, window: &Window, cx: &App) -> impl IntoElement {
    let dark = matches!(
      cx.window_appearance(),
      WindowAppearance::Dark | WindowAppearance::VibrantDark
    );
    let (bg, mark) = if dark {
      (rgb(0x1e1e1e), rgb(0xffffff))
    } else {
      (rgb(0xf3f3f3), rgb(0x000000))
    };
    let _ = window;
    div().size_full().flex().items_center().justify_center().bg(bg).child(
      svg()
        .path("brand/deathpush.svg")
        .size(px(80.0))
        .text_color(mark)
        .opacity(0.6),
    )
  }

  fn render_toast(&self, cx: &mut Context<Self>) -> Option<impl IntoElement> {
    let message = self.toast.clone()?;
    let palette = cx.global::<ActivePalette>().0;
    Some(
      div()
        .absolute()
        .bottom(px(16.0))
        .right(px(16.0))
        .max_w(px(420.0))
        .px_3()
        .py_2()
        .rounded_md()
        .bg(hsla(palette.danger))
        .text_color(hsla(deathpush_core::theme::Rgba::rgb(255, 255, 255)))
        .text_size(px(12.0))
        .cursor_pointer()
        .on_mouse_down(
          MouseButton::Left,
          cx.listener(|this, _, _, cx| {
            this.toast = None;
            cx.notify();
          }),
        )
        .child(message),
    )
  }
}

impl Render for Shell {
  fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
    self.save_bounds_if_changed(window, cx);
    self.sync_menus(window, cx);
    let screen_context = match self.screen {
      Screen::Boot => None,
      Screen::Welcome(_) => Some(CONTEXT_WELCOME),
      Screen::Repository(_) => Some(CONTEXT_REPOSITORY),
    };
    let key_context = match screen_context {
      Some(screen) => format!("{CONTEXT_APP} {screen}"),
      None => CONTEXT_APP.to_string(),
    };
    let bar_title = if matches!(self.screen, Screen::Welcome(_)) {
      SharedString::from("")
    } else {
      self.title.clone()
    };
    let title_bar = render_title_bar(bar_title, self.menu_context(), window, cx);
    let body: AnyElement = match &self.screen {
      Screen::Boot => self.render_boot(window, cx).into_any_element(),
      Screen::Welcome(view) => view.clone().into_any_element(),
      Screen::Repository(view) => view.clone().into_any_element(),
    };
    #[allow(clippy::manual_map)]
    let overlay: Option<AnyElement> = match &self.overlay {
      None => None,
      Some(Overlay::Opening) => Some(crate::overlays::opening::render_opening(cx).into_any_element()),
      // Task 9
      // Some(Overlay::Clone(view)) => Some(view.clone().into_any_element()),
      // Some(Overlay::WorkspaceSettings(view)) => Some(view.clone().into_any_element()),
      // Some(Overlay::Licenses(view)) => Some(view.clone().into_any_element()),
    };
    div()
      .key_context(key_context.as_str())
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .flex_col()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .on_action(cx.listener(|_, _: &NewWindow, _, cx| {
        open_shell_window(None, cx);
      }))
      .on_action(cx.listener(|this, _: &OpenRepository, window, cx| this.prompt_open_repository(window, cx)))
      .on_action(cx.listener(|this, _: &CloneRepository, window, cx| this.open_clone_dialog(window, cx)))
      .on_action(cx.listener(|_, _: &CloseWindow, window, _| window.remove_window()))
      .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
      .on_action(cx.listener(|_, _: &Maximize, window, _| window.zoom_window()))
      .on_action(cx.listener(|_, _: &ZoomIn, _, cx| zoom::set_zoom_level(zoom::current_level(cx) + 1, cx)))
      .on_action(cx.listener(|_, _: &ZoomOut, _, cx| zoom::set_zoom_level(zoom::current_level(cx) - 1, cx)))
      .on_action(cx.listener(|_, _: &ZoomReset, _, cx| zoom::set_zoom_level(0, cx)))
      .on_action(cx.listener(|this, _: &OpenLicenses, window, cx| this.open_licenses(window, cx)))
      .on_action(cx.listener(|this, _: &ConfigureWorkspace, window, cx| this.open_workspace_settings(window, cx)))
      .on_action(cx.listener(|this, _: &FocusRecentFilter, window, cx| {
        if let Screen::Welcome(view) = &this.screen {
          view.update(cx, |view, cx| view.focus_recent_filter(window, cx));
        }
      }))
      .on_action(cx.listener(|this, _: &FocusWorkspaceFilter, window, cx| {
        if let Screen::Welcome(view) = &this.screen {
          view.update(cx, |view, cx| view.focus_workspace_filter(window, cx));
        }
      }))
      .on_action(cx.listener(|this, _: &InstallCli, window, cx| crate::cli_install::run(this, window, cx)))
      .on_action(cx.listener(|this, _: &About, window, cx| {
        let detail = format!("Version {} ({})", env!("CARGO_PKG_VERSION"), env!("DEATHPUSH_GIT_HASH"));
        drop(window.prompt(PromptLevel::Info, "DeathPush", Some(&detail), &["OK"], cx));
        let _ = this;
      }))
      .children(title_bar)
      .child(
        div()
          .relative()
          .flex_1()
          .min_h_0()
          .child(body)
          .children(overlay)
          .children(self.render_toast(cx)),
      )
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;
  use gpui_kit::TestAppContext;

  #[gpui_kit::test]
  fn shell_root_is_focused_after_construction(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let window = cx.add_window(|window, cx| Shell::new(core, 0, None, window, cx));
    window
      .update(cx, |shell, window, cx| {
        assert_eq!(window.focused(cx).as_ref(), Some(&shell.focus_handle));
      })
      .unwrap();
  }
}
