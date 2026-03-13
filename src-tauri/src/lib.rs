mod commands;
mod error;
mod git;
mod pty;
mod shell_env;
mod types;
mod util;

use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use commands::repository::{AppRepoState, CliPaths};
use commands::{
  blame, branch, cli, commit, config, explorer, file_ops, lifecycle, log, remote, repository, staging, stash, status,
  tag, terminal,
};
use deathpush_core::events::EventSink;
use git::watcher::WatcherState;
use tauri::menu::{MenuBuilder, MenuItem, MenuItemBuilder, SubmenuBuilder};
use tauri::webview::WebviewWindowBuilder;
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WindowEvent};

static SHARED_EVENT_SINK: OnceLock<Arc<dyn EventSink>> = OnceLock::new();

pub fn get_event_sink() -> Option<Arc<dyn EventSink>> {
  SHARED_EVENT_SINK.get().cloned()
}

struct TauriEventSink {
  app_handle: AppHandle,
}

impl EventSink for TauriEventSink {
  fn emit_git_command(&self, command: &str, duration_ms: u64, timestamp: &str) {
    #[derive(serde::Serialize, Clone)]
    struct GitCommandEvent {
      command: String,
      duration_ms: u64,
      timestamp: String,
    }
    let _ = self.app_handle.emit("git:command", GitCommandEvent {
      command: command.to_string(),
      duration_ms,
      timestamp: timestamp.to_string(),
    });
  }

  fn emit_repository_changed(&self, session_id: &str) {
    if let Some(w) = self.app_handle.get_webview_window(session_id) {
      let _ = w.emit("repository-changed", ());
    }
  }

  fn emit_watcher_error(&self, session_id: &str, message: &str) {
    if let Some(w) = self.app_handle.get_webview_window(session_id) {
      let _ = w.emit("watcher-error", message);
    }
  }
}

struct RepoMenuItems(Vec<MenuItem<tauri::Wry>>);
struct RepoWindowFlags(Mutex<HashMap<String, bool>>);
struct LastFocusedWindow(Mutex<Option<String>>);
struct ConfirmedCloseWindows(Mutex<HashSet<String>>);

use tauri_plugin_deep_link::DeepLinkExt;

static WINDOW_COUNTER: AtomicU64 = AtomicU64::new(1);

fn extract_path_from_url(url: &url::Url) -> Option<String> {
  let path = url.path().to_string();
  if !path.is_empty() && std::path::Path::new(&path).is_dir() {
    Some(path)
  } else {
    None
  }
}

fn build_window(app_handle: &AppHandle, label: &str) -> Result<tauri::WebviewWindow, tauri::Error> {
  #[allow(unused_mut)]
  let mut builder = WebviewWindowBuilder::new(app_handle, label, WebviewUrl::App("index.html".into()))
    .title("DeathPush")
    .inner_size(1400.0, 900.0)
    .min_inner_size(640.0, 480.0)
    .background_color(Color(30, 30, 30, 255));

  #[cfg(target_os = "macos")]
  {
    builder = builder
      .title_bar_style(tauri::TitleBarStyle::Overlay)
      .hidden_title(true);
  }

  #[cfg(target_os = "linux")]
  {
    builder = builder.decorations(false);
  }

  builder.build()
}

fn create_window(app_handle: &AppHandle) -> Result<tauri::WebviewWindow, tauri::Error> {
  let id = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
  let window = build_window(app_handle, &format!("main-{}", id))?;

  #[cfg(target_os = "linux")]
  hide_gtk_menu_bar(&window);

  Ok(window)
}

#[tauri::command]
fn new_window(app: AppHandle, path: Option<String>) -> Result<(), error::Error> {
  let window = create_window(&app).map_err(|e| error::Error::other(e.to_string()))?;
  if let Some(p) = path {
    if let Some(state) = app.try_state::<CliPaths>() {
      if let Ok(mut map) = state.paths.lock() {
        map.insert(window.label().to_string(), p);
      }
    }
  }
  Ok(())
}

#[tauri::command]
fn set_repo_menu_enabled(app: AppHandle, window: tauri::WebviewWindow, enabled: bool) -> Result<(), error::Error> {
  let flags = app.state::<RepoWindowFlags>();
  {
    let mut map = flags.0.lock().map_err(|e| error::Error::other(e.to_string()))?;
    map.insert(window.label().to_string(), enabled);
  }
  // Only enable menu items if the focused window has a repo
  if window.is_focused().unwrap_or(false) {
    let items = app.state::<RepoMenuItems>();
    for item in &items.0 {
      item
        .set_enabled(enabled)
        .map_err(|e| error::Error::other(e.to_string()))?;
    }
  }
  Ok(())
}

fn update_menu_for_focused_window(app_handle: &AppHandle) {
  let Some(flags) = app_handle.try_state::<RepoWindowFlags>() else {
    return;
  };
  let Some(items) = app_handle.try_state::<RepoMenuItems>() else {
    return;
  };
  let Some(last) = app_handle.try_state::<LastFocusedWindow>() else {
    return;
  };

  let enabled = last
    .0
    .lock()
    .ok()
    .and_then(|guard| {
      let label = guard.as_ref()?;
      let map = flags.0.lock().ok()?;
      Some(*map.get(label).unwrap_or(&false))
    })
    .unwrap_or(false);

  for item in &items.0 {
    let _ = item.set_enabled(enabled);
  }
}

#[tauri::command]
fn set_native_theme(app: AppHandle, dark: bool) -> Result<(), error::Error> {
  // On Linux, skip set_theme() to avoid GTK menu bar contrast issues.
  // set_theme(Dark) makes GTK render light text on menu items, but the system
  // compositor still draws the menu bar background using the system's light theme.
  #[cfg(target_os = "linux")]
  {
    let _ = (&app, dark);
    Ok(())
  }

  #[cfg(not(target_os = "linux"))]
  {
    let theme = if dark {
      Some(tauri::Theme::Dark)
    } else {
      Some(tauri::Theme::Light)
    };
    for window in app.webview_windows().values() {
      window
        .set_theme(theme)
        .map_err(|e| error::Error::other(e.to_string()))?;
    }
    Ok(())
  }
}

#[tauri::command]
fn quit_app(app: AppHandle) {
  app.exit(0);
}

#[tauri::command]
fn window_minimize(window: tauri::WebviewWindow) -> Result<(), error::Error> {
  window.minimize().map_err(|e| error::Error::other(e.to_string()))
}

#[tauri::command]
fn window_maximize(window: tauri::WebviewWindow) -> Result<(), error::Error> {
  if window.is_maximized().unwrap_or(false) {
    window.unmaximize().map_err(|e| error::Error::other(e.to_string()))
  } else {
    window.maximize().map_err(|e| error::Error::other(e.to_string()))
  }
}

#[tauri::command]
fn window_close(window: tauri::WebviewWindow) -> Result<(), error::Error> {
  window.close().map_err(|e| error::Error::other(e.to_string()))
}

#[tauri::command]
fn window_confirm_close(
  window: tauri::WebviewWindow,
  state: tauri::State<'_, ConfirmedCloseWindows>,
) -> Result<(), error::Error> {
  let mut set = state.0.lock().map_err(|e| error::Error::other(e.to_string()))?;
  set.insert(window.label().to_string());
  drop(set);
  window.close().map_err(|e| error::Error::other(e.to_string()))
}

#[cfg(target_os = "linux")]
fn hide_gtk_menu_bar(window: &tauri::WebviewWindow) {
  use gtk::prelude::*;
  if let Ok(vbox) = window.default_vbox() {
    for child in vbox.children() {
      if child.is::<gtk::MenuBar>() {
        child.hide();
        child.set_no_show_all(true);
        break;
      }
    }
  }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tracing_subscriber::fmt::init();
  shell_env::init();

  let mut builder = tauri::Builder::default()
    .manage(Mutex::new(AppRepoState::default()))
    .manage(pty::TerminalState::new(HashMap::new()))
    .manage(WatcherState::new(HashMap::new()))
    .setup(|app| {
      let mut initial_paths = HashMap::new();

      // Direct binary invocation: ./deathpush /path
      if let Some(p) = std::env::args().nth(1) {
        if std::path::Path::new(&p).is_dir() {
          initial_paths.insert("main".to_string(), p);
        }
      }

      // Deep link launch: open deathpush:///path (macOS delivers URL that launched the app)
      #[cfg(target_os = "macos")]
      if let Ok(Some(urls)) = app.deep_link().get_current() {
        if let Some(path) = urls.first().and_then(extract_path_from_url) {
          initial_paths.insert("main".to_string(), path);
        }
      }

      app.manage(CliPaths {
        paths: Mutex::new(initial_paths),
      });

      // Handle deep links while app is already running.
      // The callback runs on the main thread (macOS Apple Event handler), so we must
      // spawn the window creation asynchronously to avoid deadlocking the event loop.
      let handle = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        if let Some(url) = urls.first() {
          if let Some(path) = extract_path_from_url(url) {
            let h = handle.clone();
            std::thread::spawn(move || match create_window(&h) {
              Ok(window) => {
                if let Some(state) = h.try_state::<CliPaths>() {
                  if let Ok(mut map) = state.paths.lock() {
                    map.insert(window.label().to_string(), path);
                  }
                }
              }
              Err(e) => tracing::error!("failed to create window for deep link: {:?}", e),
            });
          }
        }
      });

      let _ = SHARED_EVENT_SINK.set(Arc::new(TauriEventSink { app_handle: app.handle().clone() }));

      let settings_item = MenuItemBuilder::with_id("preferences", "Settings...")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
      #[cfg(not(target_os = "linux"))]
      let install_cli_item = MenuItemBuilder::with_id("install-cli", "Install Command Line Tool...").build(app)?;

      let open_repo_item = MenuItemBuilder::with_id("open-repo", "Open Repository...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
      let clone_repo_item = MenuItemBuilder::with_id("clone-repo", "Clone Repository...").build(app)?;
      let new_window_item = MenuItemBuilder::with_id("new-window", "New Window")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;

      let changes_item = MenuItemBuilder::with_id("view-changes", "Changes")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
      let history_item = MenuItemBuilder::with_id("view-history", "History")
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
      let quick_open_item = MenuItemBuilder::with_id("quick-open", "Quick Open...")
        .accelerator("CmdOrCtrl+P")
        .build(app)?;
      let toggle_diff_item = MenuItemBuilder::with_id("toggle-diff", "Toggle Diff Mode")
        .accelerator("CmdOrCtrl+Shift+P")
        .build(app)?;

      let zoom_in_item = MenuItemBuilder::with_id("zoom-in", "Zoom In")
        .accelerator("CmdOrCtrl+=")
        .build(app)?;
      let zoom_out_item = MenuItemBuilder::with_id("zoom-out", "Zoom Out")
        .accelerator("CmdOrCtrl+-")
        .build(app)?;
      let zoom_reset_item = MenuItemBuilder::with_id("zoom-reset", "Reset Zoom")
        .accelerator("CmdOrCtrl+0")
        .build(app)?;

      let color_theme_item = MenuItemBuilder::with_id("color-theme", "Color Theme...").build(app)?;
      let icon_theme_item = MenuItemBuilder::with_id("icon-theme", "File Icon Theme...").build(app)?;

      let licenses_item = MenuItemBuilder::with_id("open-source-licenses", "Open Source Licenses").build(app)?;

      let new_terminal_item = MenuItemBuilder::with_id("new-terminal", "New Terminal")
        .accelerator("CmdOrCtrl+Shift+J")
        .build(app)?;
      let kill_terminal_item = MenuItemBuilder::with_id("kill-terminal", "Kill Terminal").build(app)?;
      let toggle_terminal_item = MenuItemBuilder::with_id("toggle-terminal", "Toggle Terminal")
        .accelerator("CmdOrCtrl+J")
        .build(app)?;

      // DeathPush
      #[allow(unused_mut)]
      let mut app_builder = SubmenuBuilder::new(app, "DeathPush")
        .about(None)
        .separator()
        .item(&settings_item);

      #[cfg(not(target_os = "linux"))]
      {
        app_builder = app_builder.item(&install_cli_item);
      }

      app_builder = app_builder.separator();

      #[cfg(target_os = "macos")]
      {
        app_builder = app_builder
          .services()
          .separator()
          .hide()
          .hide_others()
          .show_all()
          .separator();
      }

      let app_submenu = app_builder.quit().build()?;

      // File
      let file_submenu = SubmenuBuilder::new(app, "File")
        .item(&new_window_item)
        .separator()
        .item(&open_repo_item)
        .item(&clone_repo_item)
        .separator()
        .close_window()
        .build()?;

      // Edit
      let edit_submenu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .build()?;

      // View
      #[allow(unused_mut)]
      let mut view_builder = SubmenuBuilder::new(app, "View")
        .item(&quick_open_item)
        .separator()
        .item(&changes_item)
        .item(&history_item)
        .separator()
        .item(&toggle_diff_item)
        .separator()
        .item(&color_theme_item)
        .item(&icon_theme_item)
        .separator()
        .item(&zoom_in_item)
        .item(&zoom_out_item)
        .item(&zoom_reset_item);

      #[cfg(debug_assertions)]
      let inspect_item = MenuItemBuilder::with_id("inspect", "Inspect Element")
        .accelerator("CmdOrCtrl+Shift+I")
        .build(app)?;

      #[cfg(debug_assertions)]
      {
        view_builder = view_builder.separator().item(&inspect_item);
      }

      let view_submenu = view_builder.build()?;

      // Git
      let git_pull_item = MenuItemBuilder::with_id("git-pull", "Pull").build(app)?;
      let git_push_item = MenuItemBuilder::with_id("git-push", "Push").build(app)?;
      let git_fetch_item = MenuItemBuilder::with_id("git-fetch", "Fetch").build(app)?;
      let git_stage_all_item = MenuItemBuilder::with_id("git-stage-all", "Stage All").build(app)?;
      let git_unstage_all_item = MenuItemBuilder::with_id("git-unstage-all", "Unstage All").build(app)?;
      let git_stash_item = MenuItemBuilder::with_id("git-stash", "Stash...").build(app)?;
      let git_stash_pop_item = MenuItemBuilder::with_id("git-stash-pop", "Stash Pop").build(app)?;
      let git_undo_commit_item = MenuItemBuilder::with_id("git-undo-commit", "Undo Last Commit").build(app)?;

      let git_submenu = SubmenuBuilder::new(app, "Git")
        .item(&git_pull_item)
        .item(&git_push_item)
        .item(&git_fetch_item)
        .separator()
        .item(&git_stage_all_item)
        .item(&git_unstage_all_item)
        .separator()
        .item(&git_stash_item)
        .item(&git_stash_pop_item)
        .separator()
        .item(&git_undo_commit_item)
        .build()?;

      // Terminal
      let terminal_submenu = SubmenuBuilder::new(app, "Terminal")
        .item(&new_terminal_item)
        .item(&kill_terminal_item)
        .separator()
        .item(&toggle_terminal_item)
        .build()?;

      // Window
      let window_submenu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .maximize()
        .separator()
        .close_window()
        .build()?;

      // Help
      let help_submenu = SubmenuBuilder::new(app, "Help").item(&licenses_item).build()?;

      let menu = MenuBuilder::new(app)
        .item(&app_submenu)
        .item(&file_submenu)
        .item(&edit_submenu)
        .item(&view_submenu)
        .item(&git_submenu)
        .item(&terminal_submenu)
        .item(&window_submenu)
        .item(&help_submenu)
        .build()?;

      app.set_menu(menu)?;

      let repo_items: Vec<MenuItem<tauri::Wry>> = vec![
        settings_item.clone(),
        changes_item.clone(),
        history_item.clone(),
        toggle_diff_item.clone(),
        git_pull_item.clone(),
        git_push_item.clone(),
        git_fetch_item.clone(),
        git_stage_all_item.clone(),
        git_unstage_all_item.clone(),
        git_stash_item.clone(),
        git_stash_pop_item.clone(),
        git_undo_commit_item.clone(),
        new_terminal_item.clone(),
        kill_terminal_item.clone(),
        toggle_terminal_item.clone(),
        quick_open_item.clone(),
      ];
      for item in &repo_items {
        let _ = item.set_enabled(false);
      }
      app.manage(RepoMenuItems(repo_items));
      app.manage(RepoWindowFlags(Mutex::new(HashMap::new())));
      app.manage(LastFocusedWindow(Mutex::new(None)));
      app.manage(ConfirmedCloseWindows(Mutex::new(HashSet::new())));

      #[cfg(target_os = "linux")]
      for window in app.webview_windows().values() {
        let _ = window.set_decorations(false);
        hide_gtk_menu_bar(window);
      }

      app.on_menu_event(move |app_handle, event| {
        let id = event.id();

        // Handle new window directly -- no need to forward to frontend
        if id == new_window_item.id() {
          let _ = create_window(app_handle);
          return;
        }

        // Use last focused window, then try currently focused, then fall back to any window
        let windows = app_handle.webview_windows();
        let window = app_handle
          .try_state::<LastFocusedWindow>()
          .and_then(|state| {
            let label = state.0.lock().ok()?.clone()?;
            windows.get(&label).cloned()
          })
          .or_else(|| windows.values().find(|w| w.is_focused().unwrap_or(false)).cloned())
          .or_else(|| windows.values().next().cloned());

        if let Some(window) = window {
          #[cfg(debug_assertions)]
          if id == inspect_item.id() {
            window.open_devtools();
            return;
          }

          if id == settings_item.id()
            || id == open_repo_item.id()
            || id == clone_repo_item.id()
            || id == changes_item.id()
            || id == history_item.id()
            || id == new_terminal_item.id()
            || id == kill_terminal_item.id()
            || id == toggle_terminal_item.id()
            || id == toggle_diff_item.id()
            || id == zoom_in_item.id()
            || id == zoom_out_item.id()
            || id == zoom_reset_item.id()
            || id == color_theme_item.id()
            || id == icon_theme_item.id()
            || id == git_pull_item.id()
            || id == git_push_item.id()
            || id == git_fetch_item.id()
            || id == git_stage_all_item.id()
            || id == git_unstage_all_item.id()
            || id == git_stash_item.id()
            || id == git_stash_pop_item.id()
            || id == git_undo_commit_item.id()
            || id == quick_open_item.id()
            || id == licenses_item.id()
          {
            let _ = window.emit_to(window.label(), &format!("menu:{}", id.0), ());
          }

          #[cfg(not(target_os = "linux"))]
          if id == install_cli_item.id() {
            let _ = window.emit_to(window.label(), &format!("menu:{}", id.0), ());
          }
        }
      });

      Ok(())
    })
    .on_window_event(|window, event| {
      if let WindowEvent::Focused(true) = event {
        let app_handle = window.app_handle();
        if let Some(state) = app_handle.try_state::<LastFocusedWindow>() {
          if let Ok(mut guard) = state.0.lock() {
            *guard = Some(window.label().to_string());
          }
        }
        update_menu_for_focused_window(app_handle);
        // Catch external changes missed by file watcher (network FS, etc.)
        let _ = window.emit("repository-changed", ());
      }
      if let WindowEvent::CloseRequested { api, .. } = event {
        let app_handle = window.app_handle();
        let confirmed = app_handle
          .try_state::<ConfirmedCloseWindows>()
          .and_then(|state| {
            let mut set = state.0.lock().ok()?;
            Some(set.remove(window.label()))
          })
          .unwrap_or(false);
        if !confirmed {
          api.prevent_close();
          let _ = window.emit("window:close-requested", ());
        }
      }
      if let WindowEvent::Destroyed = event {
        let label = window.label().to_string();
        let app_handle = window.app_handle();

        // Clean up repo state
        if let Some(state) = app_handle.try_state::<Mutex<AppRepoState>>() {
          if let Ok(mut guard) = state.lock() {
            guard.remove(&label);
          }
        }

        // Clean up watcher
        if let Some(state) = app_handle.try_state::<WatcherState>() {
          if let Ok(mut guard) = state.lock() {
            guard.remove(&label); // Drop sends stop signal
          }
        }

        // Clean up repo menu flags
        if let Some(flags) = app_handle.try_state::<RepoWindowFlags>() {
          if let Ok(mut map) = flags.0.lock() {
            map.remove(&label);
          }
        }

        // Clean up terminal sessions for this window
        if let Some(state) = app_handle.try_state::<pty::TerminalState>() {
          if let Ok(mut guard) = state.lock() {
            guard.retain(|_, session| session.session_label != label);
          }
        }

        // Clean up confirmed close set
        if let Some(state) = app_handle.try_state::<ConfirmedCloseWindows>() {
          if let Ok(mut set) = state.0.lock() {
            set.remove(&label);
          }
        }

        // Clear last focused window if it was this one
        if let Some(state) = app_handle.try_state::<LastFocusedWindow>() {
          if let Ok(mut guard) = state.0.lock() {
            if guard.as_deref() == Some(&label) {
              *guard = None;
            }
          }
        }
      }
    })
    .plugin(tauri_plugin_window_state::Builder::new().build())
    .plugin(tauri_plugin_deep_link::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_shell::init())
    .plugin(tauri_plugin_process::init());

  if !cfg!(dev) {
    builder = builder.plugin(tauri_plugin_updater::Builder::new().build());
  }

  builder
    .invoke_handler(tauri::generate_handler![
      repository::open_repository,
      repository::get_initial_path,
      repository::scan_projects_directory,
      repository::discover_repositories,
      repository::detect_worktrees,
      repository::get_repo_branch,
      status::get_status,
      status::get_file_diff,
      staging::stage_files,
      staging::stage_all,
      staging::unstage_files,
      staging::unstage_all,
      staging::discard_changes,
      staging::get_file_hunks,
      staging::stage_hunk,
      staging::discard_hunk,
      staging::stage_lines,
      commit::commit,
      branch::list_branches,
      branch::checkout_branch,
      branch::create_branch,
      branch::delete_branch,
      branch::rename_branch,
      branch::delete_remote_branch,
      remote::push,
      remote::pull,
      remote::fetch,
      log::get_commit_log,
      log::get_commit_detail,
      log::get_commit_file_diff,
      stash::get_last_commit_message,
      stash::undo_last_commit,
      stash::stash_save,
      stash::stash_list,
      stash::stash_apply,
      stash::stash_pop,
      stash::stash_drop,
      stash::stash_save_include_untracked,
      stash::stash_save_staged,
      stash::stash_show,
      tag::list_tags,
      tag::create_tag,
      tag::delete_tag,
      tag::push_tag,
      tag::delete_remote_tag,
      explorer::list_directory,
      explorer::read_file_content,
      explorer::fuzzy_find_files,
      explorer::search_file_contents,
      file_ops::open_in_editor,
      file_ops::reveal_in_file_manager,
      file_ops::add_to_gitignore,
      file_ops::write_file,
      file_ops::delete_file,
      file_ops::delete_files,
      file_ops::rename_entry,
      file_ops::create_directory,
      file_ops::copy_entries,
      file_ops::move_entries,
      file_ops::duplicate_entry,
      file_ops::import_files,
      lifecycle::clone_repository,
      lifecycle::init_repository,
      lifecycle::merge_branch,
      lifecycle::merge_continue,
      lifecycle::merge_abort,
      lifecycle::rebase_branch,
      lifecycle::rebase_continue,
      lifecycle::rebase_abort,
      lifecycle::rebase_skip,
      lifecycle::cherry_pick,
      lifecycle::reset_to_commit,
      terminal::terminal_spawn,
      terminal::terminal_write,
      terminal::terminal_resize,
      terminal::terminal_kill,
      terminal::terminal_foreground_process,
      config::get_git_config,
      config::set_git_config,
      blame::get_file_blame,
      blame::get_file_log,
      blame::get_last_commit_info,
      cli::check_cli_installed,
      cli::install_cli,
      cli::uninstall_cli,
      new_window,
      set_repo_menu_enabled,
      set_native_theme,
      quit_app,
      window_minimize,
      window_maximize,
      window_close,
      window_confirm_close,
    ])
    .build(tauri::generate_context!())
    .expect("error while building tauri application")
    .run(|_app_handle, event| {
      #[allow(clippy::single_match)]
      match event {
        #[cfg(target_os = "macos")]
        tauri::RunEvent::ExitRequested { api, .. } => {
          api.prevent_exit();
        }
        #[cfg(target_os = "macos")]
        tauri::RunEvent::Reopen {
          has_visible_windows, ..
        } => {
          if !has_visible_windows {
            let _ = build_window(_app_handle, "main");
          }
        }
        _ => {}
      }
    });
}
