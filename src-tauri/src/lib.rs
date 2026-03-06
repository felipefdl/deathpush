mod commands;
mod error;
mod git;
mod pty;
mod types;

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use commands::repository::{AppRepoState, CliPaths};
use commands::{blame, branch, cli, commit, config, file_ops, lifecycle, log, remote, repository, staging, stash, status, tag, terminal};
use git::watcher::WatcherState;
use tauri::menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder};
use tauri::webview::{WebviewWindowBuilder};
use tauri::window::Color;
use tauri::{AppHandle, Emitter, Manager, WebviewUrl, WindowEvent};
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

fn create_window(app_handle: &AppHandle) -> Result<tauri::WebviewWindow, tauri::Error> {
  let id = WINDOW_COUNTER.fetch_add(1, Ordering::Relaxed);
  let label = format!("main-{}", id);
  WebviewWindowBuilder::new(app_handle, &label, WebviewUrl::App("index.html".into()))
    .title("DeathPush")
    .inner_size(1200.0, 800.0)
    .min_inner_size(640.0, 480.0)
    .title_bar_style(tauri::TitleBarStyle::Overlay)
    .hidden_title(true)
    .background_color(Color(30, 30, 30, 255))
    .build()
}

#[tauri::command]
fn new_window(app: AppHandle) -> Result<(), error::Error> {
  create_window(&app).map_err(|e| error::Error::Other(e.to_string()))?;
  Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tracing_subscriber::fmt::init();

  tauri::Builder::default()
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

      app.manage(CliPaths { paths: Mutex::new(initial_paths) });

      // Handle deep links while app is already running.
      // The callback runs on the main thread (macOS Apple Event handler), so we must
      // spawn the window creation asynchronously to avoid deadlocking the event loop.
      let handle = app.handle().clone();
      app.deep_link().on_open_url(move |event| {
        let urls = event.urls();
        if let Some(url) = urls.first() {
          if let Some(path) = extract_path_from_url(url) {
            let h = handle.clone();
            std::thread::spawn(move || {
              match create_window(&h) {
                Ok(window) => {
                  if let Some(state) = h.try_state::<CliPaths>() {
                    if let Ok(mut map) = state.paths.lock() {
                      map.insert(window.label().to_string(), path);
                    }
                  }
                }
                Err(e) => tracing::error!("failed to create window for deep link: {:?}", e),
              }
            });
          }
        }
      });

      git::cli::set_app_handle(app.handle().clone());

      let settings_item = MenuItemBuilder::with_id("preferences", "Settings...")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;
      let install_cli_item = MenuItemBuilder::with_id("install-cli", "Install Command Line Tool...")
        .build(app)?;

      let open_repo_item = MenuItemBuilder::with_id("open-repo", "Open Repository...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
      let clone_repo_item = MenuItemBuilder::with_id("clone-repo", "Clone Repository...")
        .build(app)?;
      let new_window_item = MenuItemBuilder::with_id("new-window", "New Window")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;

      let changes_item = MenuItemBuilder::with_id("view-changes", "Changes")
        .accelerator("CmdOrCtrl+1")
        .build(app)?;
      let history_item = MenuItemBuilder::with_id("view-history", "History")
        .accelerator("CmdOrCtrl+2")
        .build(app)?;
      let toggle_diff_item = MenuItemBuilder::with_id("toggle-diff", "Toggle Diff Mode")
        .accelerator("CmdOrCtrl+Shift+P")
        .build(app)?;

      let new_terminal_item = MenuItemBuilder::with_id("new-terminal", "New Terminal")
        .accelerator("CmdOrCtrl+Shift+`")
        .build(app)?;
      let kill_terminal_item = MenuItemBuilder::with_id("kill-terminal", "Kill Terminal")
        .build(app)?;
      let toggle_terminal_item = MenuItemBuilder::with_id("toggle-terminal", "Toggle Terminal")
        .accelerator("CmdOrCtrl+`")
        .build(app)?;

      // DeathPush
      let app_submenu = SubmenuBuilder::new(app, "DeathPush")
        .about(None)
        .separator()
        .item(&settings_item)
        .item(&install_cli_item)
        .separator()
        .services()
        .separator()
        .hide()
        .hide_others()
        .show_all()
        .separator()
        .quit()
        .build()?;

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
      let view_submenu = SubmenuBuilder::new(app, "View")
        .item(&changes_item)
        .item(&history_item)
        .separator()
        .item(&toggle_diff_item)
        .build()?;

      // Git
      let git_pull_item = MenuItemBuilder::with_id("git-pull", "Pull").build(app)?;
      let git_push_item = MenuItemBuilder::with_id("git-push", "Push").build(app)?;
      let git_fetch_item = MenuItemBuilder::with_id("git-fetch", "Fetch").build(app)?;
      let git_stage_all_item = MenuItemBuilder::with_id("git-stage-all", "Stage All").build(app)?;
      let git_unstage_all_item = MenuItemBuilder::with_id("git-unstage-all", "Unstage All").build(app)?;
      let git_stash_item = MenuItemBuilder::with_id("git-stash", "Stash...").build(app)?;
      let git_stash_pop_item = MenuItemBuilder::with_id("git-stash-pop", "Stash Pop").build(app)?;
      let git_undo_commit_item =
        MenuItemBuilder::with_id("git-undo-commit", "Undo Last Commit").build(app)?;

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
      let help_submenu = SubmenuBuilder::new(app, "Help")
        .build()?;

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

      app.on_menu_event(move |app_handle, event| {
        let id = event.id();

        // Handle new window directly -- no need to forward to frontend
        if id == new_window_item.id() {
          let _ = create_window(app_handle);
          return;
        }

        // Find focused window, or fall back to any window
        let window = app_handle
          .webview_windows()
          .values()
          .find(|w| w.is_focused().unwrap_or(false))
          .cloned()
          .or_else(|| app_handle.webview_windows().values().next().cloned());

        if let Some(window) = window {
          if id == settings_item.id()
            || id == open_repo_item.id()
            || id == clone_repo_item.id()
            || id == changes_item.id()
            || id == history_item.id()
            || id == new_terminal_item.id()
            || id == kill_terminal_item.id()
            || id == toggle_terminal_item.id()
            || id == toggle_diff_item.id()
            || id == git_pull_item.id()
            || id == git_push_item.id()
            || id == git_fetch_item.id()
            || id == git_stage_all_item.id()
            || id == git_unstage_all_item.id()
            || id == git_stash_item.id()
            || id == git_stash_pop_item.id()
            || id == git_undo_commit_item.id()
            || id == install_cli_item.id()
          {
            let _ = window.emit(&format!("menu:{}", id.0), ());
          }
        }
      });

      Ok(())
    })
    .on_window_event(|window, event| {
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

        // Clean up terminal sessions for this window
        if let Some(state) = app_handle.try_state::<pty::TerminalState>() {
          if let Ok(mut guard) = state.lock() {
            guard.retain(|_, session| session.window_label != label);
          }
        }
      }
    })
    .plugin(tauri_plugin_deep_link::init())
    .plugin(tauri_plugin_dialog::init())
    .plugin(tauri_plugin_shell::init())
    .invoke_handler(tauri::generate_handler![
      repository::open_repository,
      repository::get_initial_path,
      repository::scan_projects_directory,
      status::get_status,
      status::get_file_diff,
      staging::stage_files,
      staging::stage_all,
      staging::unstage_files,
      staging::unstage_all,
      staging::discard_changes,
      staging::get_file_hunks,
      staging::stage_hunk,
      commit::commit,
      branch::list_branches,
      branch::checkout_branch,
      branch::create_branch,
      branch::delete_branch,
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
      tag::list_tags,
      tag::create_tag,
      tag::delete_tag,
      tag::push_tag,
      file_ops::open_in_editor,
      file_ops::reveal_in_file_manager,
      file_ops::add_to_gitignore,
      file_ops::write_file,
      file_ops::delete_file,
      lifecycle::clone_repository,
      lifecycle::merge_continue,
      lifecycle::merge_abort,
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
        tauri::RunEvent::Reopen { has_visible_windows, .. } => {
          if !has_visible_windows {
            let _ = create_window(_app_handle);
          }
        }
        _ => {}
      }
    });
}
