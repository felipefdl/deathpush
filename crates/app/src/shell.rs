use std::future::Future;
use std::path::PathBuf;
use std::sync::Arc;

use deathpush_core::config::windows::SavedWindow;
use deathpush_core::session::types::{Intent, IntentOutcome};
use deathpush_core::types::PathChangeKind;
use deathpush_core::{Core, CoreEvent, SessionId};
use futures::StreamExt;
use gpui_kit::component::ActiveTheme;
use gpui_kit::*;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::actions::*;
use crate::config::AppConfig;
use crate::keymap::{CONTEXT_APP, CONTEXT_REPOSITORY, CONTEXT_WELCOME};
use crate::menus::{MenuContext, set_menu_context};
use crate::theme::{ActivePalette, apply_for_appearance, hsla};
use crate::title_bar::render_title_bar;
use crate::updater::{CHECK_DELAY, UpdateCheck, UpdaterOps, UpdaterState, should_check_on_launch, take_ops};
use crate::window::open_shell_window;
use crate::zoom;

fn should_prompt_before_close(active_process: bool, prompt_pending: bool) -> bool {
  active_process && !prompt_pending
}

pub enum Screen {
  Boot,
  Welcome(Entity<crate::welcome::WelcomeView>),
  Repository(Entity<crate::repo::RepoView>),
}

pub enum Overlay {
  Opening,
  Clone(Entity<crate::overlays::clone_dialog::CloneDialog>),
  WorkspaceSettings(Entity<crate::overlays::workspace_settings::WorkspaceSettingsDialog>),
  Licenses(Entity<crate::overlays::licenses::LicensesDialog>),
  QuickOpen(Entity<crate::overlays::quick_open::QuickOpen>),
  BranchPicker(Entity<crate::overlays::branch_picker::BranchPicker>),
  ThemePicker(Entity<crate::overlays::theme_picker::ThemePicker>),
}

async fn watch_install<T>(
  task: impl Future<Output = T>,
  mut rx: futures::channel::mpsc::UnboundedReceiver<u8>,
  mut on_percent: impl FnMut(u8),
) -> T {
  let mut task = std::pin::pin!(task);
  loop {
    match futures::future::select(rx.next(), task.as_mut()).await {
      futures::future::Either::Left((Some(percent), _)) => on_percent(percent),
      futures::future::Either::Left((None, rest)) => return rest.await,
      futures::future::Either::Right((result, _)) => return result,
    }
  }
}

async fn run_updater_check(ops: Arc<dyn UpdaterOps>, current: String, core: Arc<Core>) -> UpdateCheck {
  if ops.blocking() {
    let (tx, rx) = futures::channel::oneshot::channel();
    core.runtime_handle().spawn_blocking(move || {
      let _ = tx.send(ops.check(&current));
    });
    rx.await
      .unwrap_or_else(|_| UpdateCheck::Failed("updater check cancelled".into()))
  } else {
    ops.check(&current)
  }
}

async fn run_updater_install(
  ops: Arc<dyn UpdaterOps>,
  update: Box<cargo_packager_updater::Update>,
  core: Arc<Core>,
  progress_tx: futures::channel::mpsc::UnboundedSender<u8>,
) -> Result<(), String> {
  if ops.blocking() {
    let (tx, rx) = futures::channel::oneshot::channel();
    core.runtime_handle().spawn_blocking(move || {
      let on_progress = move |percent: u8| {
        let _ = progress_tx.unbounded_send(percent);
      };
      let _ = tx.send(ops.install(update, &on_progress));
    });
    rx.await.unwrap_or_else(|_| Err("updater install cancelled".into()))
  } else {
    let on_progress = move |percent: u8| {
      let _ = progress_tx.unbounded_send(percent);
    };
    ops.install(update, &on_progress)
  }
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
  overlay_restore: Option<WeakFocusHandle>,
  last_saved_bounds: Option<SavedWindow>,
  cli_installed: bool,
  opening_generation: u64,
  close_prompt_pending: bool,
  close_confirmed: bool,
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
      overlay_restore: None,
      last_saved_bounds: None,
      cli_installed,
      opening_generation: 0,
      close_prompt_pending: false,
      close_confirmed: false,
    };
    shell.listen(events, window, cx);
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
    let this = cx.weak_entity();
    window.on_window_should_close(cx, move |window, cx| {
      this
        .update(cx, |this, cx| this.request_close(window, cx))
        .unwrap_or(true)
    });
    match initial {
      Some(path) => cx.defer_in(window, move |this, window, cx| this.open_repository(path, window, cx)),
      None => cx.defer_in(window, |this, window, cx| this.show_welcome(window, cx)),
    }
    shell.focus_handle.focus(window, cx);
    shell
  }

  fn listen(&self, mut events: UnboundedReceiver<CoreEvent>, window: &mut Window, cx: &mut Context<Self>) {
    cx.spawn_in(window, async move |this, cx| {
      while let Some(event) = events.recv().await {
        let alive = this.update_in(cx, |this, window, cx| match (&event, &this.screen) {
          (CoreEvent::SessionStatus(status), Screen::Repository(view)) => {
            let model = view.read(cx).model().clone();
            model.update(cx, |model, cx| model.apply_status_event(status.clone(), cx));
          }
          (CoreEvent::GitCommand(command), Screen::Repository(view)) => {
            let output = view.read(cx).output().clone();
            output.update(cx, |output, cx| output.push(command.clone(), cx));
          }
          (CoreEvent::PathsChanged(event), Screen::Repository(view)) => {
            let explorer = view.read(cx).explorer_model().clone();
            explorer.update(cx, |model, cx| model.on_paths_changed(event, cx));
            if matches!(event.kind, PathChangeKind::Content | PathChangeKind::Structural) {
              let model = view.read(cx).model().clone();
              let open_path = model.read(cx).state().open_file.as_ref().map(|open| open.path.clone());
              if let Some(path) = open_path
                && event.paths.iter().any(|changed| changed == &path)
              {
                model.update(cx, |model, cx| model.reload_open_file(cx));
              }
            }
          }
          (CoreEvent::WatcherError(message), _) => this.show_toast(message.clone(), cx),
          (CoreEvent::TerminalData { id, data }, Screen::Repository(view)) => {
            let terminal = view.read(cx).terminal().clone();
            let id = *id;
            let data = data.clone();
            terminal.update(cx, |model, _| model.on_data(id, &data));
          }
          (CoreEvent::TerminalExited { id }, Screen::Repository(view)) => {
            let terminal = view.read(cx).terminal().clone();
            let id = *id;
            terminal.update(cx, |model, cx| model.on_exited(id, Some(window), cx));
          }
          _ => {}
        });
        if alive.is_err() {
          break;
        }
      }
    })
    .detach();
  }

  fn shutdown_terminals(&self, cx: &mut Context<Self>) {
    if let Screen::Repository(view) = &self.screen {
      let terminal = view.read(cx).terminal().clone();
      terminal.update(cx, |model, cx| model.shutdown(cx));
    }
  }

  fn repo_has_active_process(&self, cx: &App) -> bool {
    match &self.screen {
      Screen::Repository(view) => view.read(cx).terminal().read(cx).has_active_process(),
      _ => false,
    }
  }

  fn request_close(&mut self, window: &mut Window, cx: &mut Context<Self>) -> bool {
    self.abandon_theme_picker(cx);
    if self.close_confirmed {
      self.shutdown_terminals(cx);
      return true;
    }
    if self.close_prompt_pending {
      return false;
    }
    if should_prompt_before_close(self.repo_has_active_process(cx), self.close_prompt_pending) {
      self.close_prompt_pending = true;
      let answer = window.prompt(
        PromptLevel::Warning,
        "Confirm",
        Some("Close the window? A terminal still has a running process."),
        &["Close", "Cancel"],
        cx,
      );
      cx.spawn_in(window, async move |this, cx| match answer.await {
        Ok(0) => {
          let _ = this.update_in(cx, |this, window, cx| {
            this.close_confirmed = true;
            this.close_prompt_pending = false;
            this.shutdown_terminals(cx);
            window.remove_window();
          });
        }
        _ => {
          let _ = this.update(cx, |this, _cx| {
            this.close_prompt_pending = false;
          });
        }
      })
      .detach();
      return false;
    }
    self.shutdown_terminals(cx);
    true
  }

  fn mount_repository(
    &mut self,
    snapshot: deathpush_core::session::types::SessionSnapshot,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    let root = snapshot.repo.root.clone();
    let branch = snapshot.repo.head_branch.clone();
    let now = chrono::Utc::now().to_rfc3339();
    AppConfig::update(cx, {
      let root = root.clone();
      let branch = branch.clone();
      move |config| config.recents.add(&root, branch, &now)
    });
    self.apply_titles(&root, branch.as_deref(), window);
    let model = cx.new(|_| crate::repo::model::RepoModel::new(self.core.clone(), self.session, snapshot));
    model.update(cx, |model, cx| model.refresh_nested_repositories(cx));
    let layout_model = crate::repo::layout_model::LayoutModel::load(&root, cx);
    let layout = cx.new(|_| layout_model);
    let output = cx.new(|_| crate::repo::output_log::OutputLog::default());
    cx.subscribe_in(
      &model,
      window,
      |this, model, event: &crate::repo::model::RepoEvent, window, cx| match event {
        crate::repo::model::RepoEvent::Error(message) => this.show_toast(message.clone(), cx),
        crate::repo::model::RepoEvent::Saved { .. } => {}
        crate::repo::model::RepoEvent::Changed => {
          let state = model.read(cx).state();
          if let Some(root) = state.root() {
            let branch = state.head_branch().map(str::to_string);
            let title = deathpush_core::ops::in_window_title(root, branch.as_deref());
            if this.title.as_ref() != title {
              this.apply_titles(root, branch.as_deref(), window);
              cx.notify();
            }
          }
        }
      },
    )
    .detach();
    let view = cx.new(|cx| crate::repo::RepoView::new(model, layout, output, window, cx));
    let explorer = view.read(cx).explorer_model().clone();
    cx.subscribe(
      &explorer,
      |this, _, event: &crate::repo::explorer::ExplorerEvent, cx| match event {
        crate::repo::explorer::ExplorerEvent::Error(message) | crate::repo::explorer::ExplorerEvent::Toast(message) => {
          this.show_toast(message.clone(), cx)
        }
        _ => {}
      },
    )
    .detach();
    view.update(cx, |view, cx| view.focus(window, cx));
    self.shutdown_terminals(cx);
    self.screen = Screen::Repository(view);
    self.sync_menus(window, cx);
    cx.notify();
    window.refresh();
  }

  fn apply_titles(&mut self, root: &str, branch: Option<&str>, window: &mut Window) {
    self.title = deathpush_core::ops::in_window_title(root, branch).into();
    window.set_window_title(&deathpush_core::ops::window_title(root, branch));
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
        crate::welcome::WelcomeEvent::InstallUpdate => this.install_available_update(cx),
      },
    )
    .detach();
    self.shutdown_terminals(cx);
    self.screen = Screen::Welcome(view);
    self.title = "DeathPush".into();
    window.set_window_title("DeathPush");
    self.focus_handle.focus(window, cx);
    self.sync_menus(window, cx);
    self.schedule_update_check(cx);
    cx.notify();
  }

  pub fn rescan_welcome(&self, cx: &mut Context<Self>) {
    if let Screen::Welcome(view) = &self.screen {
      view.update(cx, |view, cx| view.rescan(cx));
    }
  }

  fn schedule_update_check(&mut self, cx: &mut Context<Self>) {
    if !should_check_on_launch(cfg!(debug_assertions)) {
      return;
    }
    self.start_update_check(cx);
  }

  pub(crate) fn start_update_check(&mut self, cx: &mut Context<Self>) {
    if !cx.update_global::<UpdaterState, _>(|state, _| state.begin_check()) {
      return;
    }
    let current = env!("CARGO_PKG_VERSION").to_string();
    let ops = take_ops(cx);
    let core = self.core.clone();
    cx.spawn(async move |this, cx| {
      cx.background_executor().timer(CHECK_DELAY).await;
      let result = run_updater_check(ops, current, core).await;
      cx.update(|cx| {
        let toast = cx.update_global::<UpdaterState, _>(|state, _| state.apply_check(result));
        if let Some(message) = toast {
          let _ = this.update(cx, |this, cx| this.show_toast(message, cx));
        }
      });
    })
    .detach();
  }

  fn install_available_update(&mut self, cx: &mut Context<Self>) {
    let Some(update) = cx.update_global::<UpdaterState, _>(|state, _| state.begin_install()) else {
      return;
    };
    let ops = take_ops(cx);
    let core = self.core.clone();
    let (progress_tx, progress_rx) = futures::channel::mpsc::unbounded();
    cx.spawn(async move |this, cx| {
      let task = run_updater_install(ops, update, core, progress_tx);
      let progress_cx = cx.clone();
      let result = watch_install(task, progress_rx, move |percent| {
        progress_cx.update_global::<UpdaterState, _>(|state, _| state.set_percent(percent));
      })
      .await;
      cx.update(|cx| {
        let toast = cx.update_global::<UpdaterState, _>(|state, _| state.finish_install(result));
        if let Some(err) = toast {
          let _ = this.update(cx, |this, cx| this.show_toast(err, cx));
        }
      });
    })
    .detach();
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
            this.mount_repository(*snapshot, window, cx);
          }
          Ok(Ok(_)) => this.show_toast("Unexpected outcome", cx),
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
      // update_in takes the window off the map, so notify cannot schedule a frame.
      cx.refresh();
    })
    .detach();
  }

  pub fn show_toast(&mut self, message: impl Into<SharedString>, cx: &mut Context<Self>) {
    self.toast = Some(message.into());
    cx.notify();
  }

  fn remember_overlay_opener(&mut self, window: &Window, cx: &App) {
    if self.overlay.is_none() && self.overlay_restore.is_none() {
      self.overlay_restore = window.focused(cx).map(|handle| handle.downgrade());
    }
  }

  pub fn set_overlay(&mut self, overlay: Option<Overlay>, window: &mut Window, cx: &mut Context<Self>) {
    self.abandon_theme_picker(cx);
    if overlay.is_some() {
      self.remember_overlay_opener(window, cx);
    }
    let closing = overlay.is_none();
    self.overlay = overlay;
    if closing {
      if let Some(handle) = self.overlay_restore.take().and_then(|handle| handle.upgrade()) {
        handle.focus(window, cx);
      } else {
        match &self.screen {
          Screen::Repository(view) => view.update(cx, |view, cx| view.focus(window, cx)),
          _ => self.focus_handle.focus(window, cx),
        }
      }
    }
    cx.notify();
  }

  fn abandon_theme_picker(&mut self, cx: &mut Context<Self>) {
    if let Some(Overlay::ThemePicker(view)) = &self.overlay
      && !view.read(cx).is_finished()
    {
      view.update(cx, |view, cx| view.finish(cx));
    }
  }

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

  pub fn open_clone_dialog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.remember_overlay_opener(window, cx);
    let dialog = cx.new(|cx| crate::overlays::clone_dialog::CloneDialog::new(window, cx));
    cx.subscribe_in(
      &dialog,
      window,
      |this, dialog, event: &crate::overlays::clone_dialog::CloneEvent, window, cx| match event {
        crate::overlays::clone_dialog::CloneEvent::Close => this.set_overlay(None, window, cx),
        crate::overlays::clone_dialog::CloneEvent::Clone { url, directory } => {
          this.clone_repository(dialog.clone(), url.clone(), directory.clone(), window, cx)
        }
      },
    )
    .detach();
    self.set_overlay(Some(Overlay::Clone(dialog)), window, cx);
  }

  fn clone_repository(
    &mut self,
    dialog: Entity<crate::overlays::clone_dialog::CloneDialog>,
    url: String,
    directory: String,
    window: &mut Window,
    cx: &mut Context<Self>,
  ) {
    dialog.update(cx, |dialog, cx| dialog.set_cloning(true, cx));
    let core = self.core.clone();
    let runtime = core.clone();
    let session = self.session;
    let task = runtime.spawn(async move {
      core
        .session_intent(session, Intent::CloneRepository { url, directory })
        .await
    });
    cx.spawn_in(window, async move |this, cx| {
      let result = task.await;
      let _ = this.update_in(cx, |this, window, cx| match result {
        Ok(Ok(IntentOutcome::Snapshot { snapshot })) => {
          this.set_overlay(None, window, cx);
          this.mount_repository(*snapshot, window, cx);
        }
        Ok(Ok(_)) => {
          dialog.update(cx, |dialog, cx| dialog.set_cloning(false, cx));
          this.show_toast("Unexpected outcome", cx);
        }
        Ok(Err(err)) => {
          dialog.update(cx, |dialog, cx| dialog.set_cloning(false, cx));
          this.show_toast(err.to_string(), cx);
        }
        Err(err) => {
          dialog.update(cx, |dialog, cx| dialog.set_cloning(false, cx));
          this.show_toast(err.to_string(), cx);
        }
      });
      cx.refresh();
    })
    .detach();
  }

  pub fn open_workspace_settings(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.remember_overlay_opener(window, cx);
    let entries = AppConfig::get(cx).settings.projects.workspaces.clone();
    let dialog = cx.new(|cx| crate::overlays::workspace_settings::WorkspaceSettingsDialog::new(&entries, window, cx));
    cx.subscribe_in(
      &dialog,
      window,
      |this, _, event: &crate::overlays::workspace_settings::WorkspaceEvent, window, cx| match event {
        crate::overlays::workspace_settings::WorkspaceEvent::Close => this.set_overlay(None, window, cx),
        crate::overlays::workspace_settings::WorkspaceEvent::Save(entries) => {
          let entries = entries.clone();
          AppConfig::update(cx, move |config| config.settings.projects.workspaces = entries);
          this.set_overlay(None, window, cx);
          this.rescan_welcome(cx);
        }
      },
    )
    .detach();
    self.set_overlay(Some(Overlay::WorkspaceSettings(dialog)), window, cx);
  }

  pub fn open_quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(Overlay::QuickOpen(view)) = &self.overlay {
      view.update(cx, |view, cx| view.focus(window, cx));
      return;
    }
    let Screen::Repository(repo) = &self.screen else {
      return;
    };
    let repo = repo.clone();
    self.remember_overlay_opener(window, cx);
    let overlay = cx.new(|cx| crate::overlays::quick_open::QuickOpen::new(repo, window, cx));
    cx.subscribe_in(
      &overlay,
      window,
      |this, _, _: &crate::overlays::quick_open::QuickOpenEvent, window, cx| {
        this.set_overlay(None, window, cx);
      },
    )
    .detach();
    self.set_overlay(Some(Overlay::QuickOpen(overlay)), window, cx);
  }

  pub fn open_branch_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(Overlay::BranchPicker(view)) = &self.overlay {
      view.update(cx, |view, cx| view.focus(window, cx));
      return;
    }
    let Screen::Repository(repo) = &self.screen else {
      return;
    };
    let model = repo.read(cx).model().clone();
    self.remember_overlay_opener(window, cx);
    let overlay = cx.new(|cx| crate::overlays::branch_picker::BranchPicker::new(model, window, cx));
    cx.subscribe_in(
      &overlay,
      window,
      |this, _, _: &crate::overlays::branch_picker::BranchPickerEvent, window, cx| {
        this.set_overlay(None, window, cx);
      },
    )
    .detach();
    self.set_overlay(Some(Overlay::BranchPicker(overlay)), window, cx);
  }

  pub fn open_theme_picker(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    if let Some(Overlay::ThemePicker(view)) = &self.overlay {
      view.update(cx, |view, cx| view.focus(window, cx));
      return;
    }
    self.remember_overlay_opener(window, cx);
    let overlay = cx.new(|cx| crate::overlays::theme_picker::ThemePicker::new(window, cx));
    cx.subscribe_in(
      &overlay,
      window,
      |this, _, _: &crate::overlays::theme_picker::ThemePickerEvent, window, cx| {
        this.set_overlay(None, window, cx);
      },
    )
    .detach();
    self.set_overlay(Some(Overlay::ThemePicker(overlay)), window, cx);
  }

  pub fn open_licenses(&mut self, window: &mut Window, cx: &mut Context<Self>) {
    self.remember_overlay_opener(window, cx);
    let dialog = cx.new(crate::overlays::licenses::LicensesDialog::new);
    dialog.update(cx, |dialog, cx| dialog.focus(window, cx));
    cx.subscribe_in(
      &dialog,
      window,
      |this, _, _: &crate::overlays::licenses::LicensesEvent, window, cx| this.set_overlay(None, window, cx),
    )
    .detach();
    self.set_overlay(Some(Overlay::Licenses(dialog)), window, cx);
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
        .text_color(hsla(palette.primary_foreground))
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
      Some(Overlay::Clone(view)) => Some(view.clone().into_any_element()),
      Some(Overlay::WorkspaceSettings(view)) => Some(view.clone().into_any_element()),
      Some(Overlay::Licenses(view)) => Some(view.clone().into_any_element()),
      Some(Overlay::QuickOpen(view)) => Some(view.clone().into_any_element()),
      Some(Overlay::BranchPicker(view)) => Some(view.clone().into_any_element()),
      Some(Overlay::ThemePicker(view)) => Some(view.clone().into_any_element()),
    };
    div()
      .key_context(key_context.as_str())
      .track_focus(&self.focus_handle)
      .size_full()
      .flex()
      .flex_col()
      .bg(cx.theme().background)
      .text_color(cx.theme().foreground)
      .on_drop::<ExternalPaths>(cx.listener(|this, paths: &ExternalPaths, window, cx| {
        let Screen::Repository(view) = &this.screen else {
          return;
        };
        let explorer = view.read(cx).explorer().clone();
        let sources: Vec<String> = paths
          .paths()
          .iter()
          .map(|path| path.to_string_lossy().into_owned())
          .collect();
        explorer.update(cx, |explorer, cx| explorer.import_external(sources, window, cx));
      }))
      .on_action(cx.listener(|_, _: &NewWindow, _, cx| {
        open_shell_window(None, cx);
      }))
      .on_action(cx.listener(|this, _: &OpenRepository, window, cx| this.prompt_open_repository(window, cx)))
      .on_action(cx.listener(|this, _: &CloneRepository, window, cx| this.open_clone_dialog(window, cx)))
      .on_action(cx.listener(|this, _: &CloseWindow, window, cx| {
        if this.request_close(window, cx) {
          window.remove_window();
        }
      }))
      .on_action(cx.listener(|_, _: &Minimize, window, _| window.minimize_window()))
      .on_action(cx.listener(|_, _: &Maximize, window, _| window.zoom_window()))
      .on_action(cx.listener(|_, _: &ZoomIn, _, cx| zoom::set_zoom_level(zoom::current_level(cx) + 1, cx)))
      .on_action(cx.listener(|_, _: &ZoomOut, _, cx| zoom::set_zoom_level(zoom::current_level(cx) - 1, cx)))
      .on_action(cx.listener(|_, _: &ZoomReset, _, cx| zoom::set_zoom_level(0, cx)))
      .on_action(cx.listener(|this, _: &QuickOpen, window, cx| this.open_quick_open(window, cx)))
      .on_action(cx.listener(|this, _: &ColorTheme, window, cx| this.open_theme_picker(window, cx)))
      .on_action(cx.listener(|this, _: &ShowBranchPicker, window, cx| this.open_branch_picker(window, cx)))
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

  use deathpush_core::session::types::{
    OperationActions, SessionActions, SessionRepo, SessionScm, SessionSelection, SessionSnapshot, SyncAction, SyncKind,
  };
  use deathpush_core::types::{RepoOperationState, StatusPhase};
  use gpui_kit::TestAppContext;

  use crate::actions::CloseWindow;
  use crate::theme::preview_theme;
  use crate::updater::{CHECK_DELAY, FakeOps, UpdateCheck, UpdaterState};

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

  #[gpui_kit::test]
  fn closing_an_overlay_restores_repository_focus(cx: &mut TestAppContext) {
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
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        let repo_handle = match &shell.screen {
          Screen::Repository(view) => view.read(cx).focus_handle.clone(),
          _ => panic!("expected a repository screen"),
        };
        assert_eq!(window.focused(cx).as_ref(), Some(&repo_handle));
        shell.open_licenses(window, cx);
        assert_ne!(window.focused(cx).as_ref(), Some(&repo_handle));
        shell.set_overlay(None, window, cx);
        assert_eq!(window.focused(cx).as_ref(), Some(&repo_handle));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn quick_open_opens_only_for_a_repository(cx: &mut TestAppContext) {
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
        shell.open_quick_open(window, cx);
        assert!(shell.overlay.is_none());
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        shell.open_quick_open(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::QuickOpen(_))));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn branch_picker_opens_only_for_a_repository(cx: &mut TestAppContext) {
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
        shell.open_branch_picker(window, cx);
        assert!(shell.overlay.is_none());
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        shell.open_branch_picker(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::BranchPicker(_))));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn theme_picker_opens_on_any_screen(cx: &mut TestAppContext) {
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
        shell.open_theme_picker(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::ThemePicker(_))));
        shell.open_theme_picker(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::ThemePicker(_))));
        shell.set_overlay(None, window, cx);
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        shell.open_theme_picker(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::ThemePicker(_))));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn replacing_the_theme_picker_restores_the_preview(cx: &mut TestAppContext) {
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
        let original_kind = cx.global::<ActivePalette>().0.kind;
        shell.open_theme_picker(window, cx);
        preview_theme("ayu-light", window, cx);
        assert_eq!(
          cx.global::<ActivePalette>().0.kind,
          deathpush_core::theme::ThemeKind::Light
        );
        shell.open_licenses(window, cx);
        assert!(matches!(shell.overlay, Some(Overlay::Licenses(_))));
        assert_eq!(cx.global::<ActivePalette>().0.kind, original_kind);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn closing_the_window_restores_a_theme_preview(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let window = cx.add_window(|window, cx| Shell::new(core, 0, None, window, cx));
    let original_kind = window
      .update(cx, |shell, window, cx| {
        let original_kind = cx.global::<ActivePalette>().0.kind;
        shell.open_theme_picker(window, cx);
        preview_theme("ayu-light", window, cx);
        assert_eq!(
          cx.global::<ActivePalette>().0.kind,
          deathpush_core::theme::ThemeKind::Light
        );
        original_kind
      })
      .unwrap();
    cx.dispatch_action(*window, CloseWindow);
    cx.update(|cx| {
      assert_eq!(cx.global::<ActivePalette>().0.kind, original_kind);
    });
  }

  #[gpui_kit::test]
  fn os_close_restores_a_theme_preview(cx: &mut TestAppContext) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let (shell, cx) = cx.add_window_view(|window, cx| Shell::new(core, 0, None, window, cx));
    let original_kind = cx.update(|window, cx| {
      let original_kind = cx.global::<ActivePalette>().0.kind;
      shell.update(cx, |shell, cx| shell.open_theme_picker(window, cx));
      preview_theme("ayu-light", window, cx);
      assert_eq!(
        cx.global::<ActivePalette>().0.kind,
        deathpush_core::theme::ThemeKind::Light
      );
      original_kind
    });
    assert!(cx.simulate_close());
    cx.update(|_, cx| {
      assert_eq!(cx.global::<ActivePalette>().0.kind, original_kind);
    });
  }

  #[test]
  fn should_prompt_before_close_skips_when_idle_or_pending() {
    assert!(!should_prompt_before_close(false, false));
    assert!(!should_prompt_before_close(false, true));
    assert!(should_prompt_before_close(true, false));
    assert!(!should_prompt_before_close(true, true));
  }

  fn inject_blocked_pane(shell: &Shell, cx: &mut Context<Shell>) -> crate::repo::terminal::pane_view::BlockedWake {
    let Screen::Repository(view) = &shell.screen else {
      panic!("expected a repository screen");
    };
    let terminal = view.read(cx).terminal().clone();
    let (handle, blocked) = crate::repo::terminal::pane_view::BlockedWake::spawn_handle();
    handle.push_bytes(b"x");
    blocked.wait_entered();
    let (_, rx) = futures::channel::mpsc::unbounded();
    terminal.update(cx, |model, cx| {
      let view = cx.new(|cx| crate::repo::terminal::pane_view::PaneView::new(1, Arc::clone(&handle), rx, cx));
      model.insert_test_pane_with_handle(view, handle, cx);
    });
    blocked
  }

  fn assert_returns_quickly(start: std::time::Instant) {
    assert!(
      start.elapsed() < std::time::Duration::from_millis(200),
      "teardown waited on a blocked pane thread"
    );
  }

  #[gpui_kit::test]
  fn repository_switch_returns_while_the_pane_thread_is_blocked(cx: &mut TestAppContext) {
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
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        let blocked = inject_blocked_pane(shell, cx);
        let start = std::time::Instant::now();
        shell.show_welcome(window, cx);
        assert_returns_quickly(start);
        drop(blocked);
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn window_close_returns_while_the_pane_thread_is_blocked(cx: &mut TestAppContext) {
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
        shell.mount_repository(snapshot(config_dir.path().to_str().unwrap()), window, cx);
        let blocked = inject_blocked_pane(shell, cx);
        let start = std::time::Instant::now();
        assert!(shell.request_close(window, cx));
        assert_returns_quickly(start);
        drop(blocked);
      })
      .unwrap();
  }

  fn boot_updater_shell(
    cx: &mut TestAppContext,
    ops: FakeOps,
  ) -> (tempfile::TempDir, tempfile::TempDir, WindowHandle<Shell>) {
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    let ops = Arc::new(ops);
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      cx.default_global::<UpdaterState>().set_ops(ops);
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let window = cx.add_window(|window, cx| Shell::new(core, 0, None, window, cx));
    (config_dir, resource_dir, window)
  }

  fn run_check(cx: &mut TestAppContext) {
    cx.executor().advance_clock(CHECK_DELAY);
    cx.run_until_parked();
  }

  #[gpui_kit::test]
  fn updater_available_shows_footer_button(cx: &mut TestAppContext) {
    let (_config, _resource, window) = boot_updater_shell(cx, FakeOps::available());
    window.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    run_check(cx);
    window
      .update(cx, |shell, _, cx| {
        assert_eq!(welcome_footer_label(shell, cx).as_deref(), Some("Update to v0.5.0"));
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn updater_failure_toasts(cx: &mut TestAppContext) {
    let mut ops = FakeOps::available();
    ops.check = UpdateCheck::Failed("operation timed out".into());
    let (_config, _resource, window) = boot_updater_shell(cx, ops);
    window.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    run_check(cx);
    window
      .update(cx, |shell, _, cx| {
        assert_eq!(shell.toast.as_deref(), Some("operation timed out"));
        assert!(cx.global::<UpdaterState>().button().is_none());
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn updater_download_progress_and_timeout_recovery(cx: &mut TestAppContext) {
    let mut ops = FakeOps::available();
    ops.percents = vec![40, 80];
    ops.install = Err("operation timed out".into());
    let (_config, _resource, window) = boot_updater_shell(cx, ops);
    window.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    run_check(cx);
    window
      .update(cx, |shell, _, cx| shell.install_available_update(cx))
      .unwrap();
    cx.run_until_parked();
    window
      .update(cx, |shell, _, cx| {
        assert_eq!(shell.toast.as_deref(), Some("operation timed out"));
        assert_eq!(
          cx.global::<UpdaterState>().button(),
          Some(("Update to v0.5.0".into(), false))
        );
      })
      .unwrap();
  }

  #[gpui_kit::test]
  fn updater_second_window_does_not_start_another_check(cx: &mut TestAppContext) {
    let ops = Arc::new(FakeOps::available());
    let config_dir = tempfile::TempDir::new().unwrap();
    let resource_dir = tempfile::TempDir::new().unwrap();
    cx.update(|cx| {
      gpui_kit::init(cx);
      AppConfig::init_at(config_dir.path().to_path_buf(), cx);
      crate::theme::init(cx);
      cx.default_global::<UpdaterState>().set_ops(ops.clone());
    });
    let core = Core::new(resource_dir.path().to_path_buf()).unwrap();
    let first = cx.add_window({
      let core = core.clone();
      move |window, cx| Shell::new(core, 0, None, window, cx)
    });
    let second = cx.add_window(|window, cx| Shell::new(core, 1, None, window, cx));
    first.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    second.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    run_check(cx);
    assert_eq!(ops.checks.load(std::sync::atomic::Ordering::SeqCst), 1);
    second
      .update(cx, |shell, _, cx| {
        assert_eq!(welcome_footer_label(shell, cx).as_deref(), Some("Update to v0.5.0"));
      })
      .unwrap();
  }

  fn welcome_footer_label(shell: &Shell, cx: &App) -> Option<String> {
    let Screen::Welcome(view) = &shell.screen else {
      panic!("expected welcome screen");
    };
    view.read(cx).update_footer().map(|footer| footer.label.to_string())
  }

  #[gpui_kit::test]
  fn updater_window_closing_mid_check_does_not_panic(cx: &mut TestAppContext) {
    let (_config, resource, window) = boot_updater_shell(cx, FakeOps::available());
    window.update(cx, |shell, _, cx| shell.start_update_check(cx)).unwrap();
    cx.dispatch_action(*window, CloseWindow);
    run_check(cx);
    let core = Core::new(resource.path().to_path_buf()).unwrap();
    let later = cx.add_window(|window, cx| Shell::new(core, 1, None, window, cx));
    later
      .update(cx, |shell, _, cx| {
        assert_eq!(welcome_footer_label(shell, cx).as_deref(), Some("Update to v0.5.0"));
      })
      .unwrap();
  }
}
