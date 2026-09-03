use gpui_kit::component::input::{Copy, Cut, Paste, Redo, SelectAll, Undo};
use gpui_kit::*;

use crate::actions::*;

/// What the focused window allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MenuContext {
  pub repo_open: bool,
  pub cli_installed: bool,
}

/// The current context, refreshed whenever a window's screen or activation changes.
#[derive(Default)]
pub struct MenuState(pub MenuContext);

impl Global for MenuState {}

const APP_NAME: &str = "deathpush";

fn repo_item(name: &str, action: impl Action, ctx: &MenuContext) -> MenuItem {
  MenuItem::action(name.to_string(), action).disabled(!ctx.repo_open)
}

fn menu(name: impl Into<SharedString>, items: Vec<MenuItem>) -> Menu {
  Menu {
    name: name.into(),
    items,
    disabled: false,
  }
}

pub fn build_menus(ctx: &MenuContext) -> Vec<Menu> {
  let mac = cfg!(target_os = "macos");
  let windows = cfg!(target_os = "windows");
  let mut app_items = vec![
    MenuItem::action(
      if mac {
        format!("About {APP_NAME}")
      } else {
        "About".to_string()
      },
      About,
    ),
    MenuItem::separator(),
  ];
  app_items.push(repo_item("Settings...", ShowSettings, ctx));
  if !cfg!(target_os = "linux") {
    let label = if ctx.cli_installed {
      "Uninstall Command Line Tool..."
    } else {
      "Install Command Line Tool..."
    };
    app_items.push(MenuItem::action(label, InstallCli));
  }
  if mac {
    app_items.push(MenuItem::separator());
    app_items.push(MenuItem::os_submenu("Services", SystemMenuType::Services));
    app_items.push(MenuItem::separator());
    app_items.push(MenuItem::action(format!("Hide {APP_NAME}"), Hide));
    app_items.push(MenuItem::action("Hide Others", HideOthers));
    app_items.push(MenuItem::action("Show All", ShowAll));
  }
  app_items.push(MenuItem::separator());
  app_items.push(MenuItem::action(
    if mac {
      format!("Quit {APP_NAME}")
    } else if windows {
      "Exit".to_string()
    } else {
      "Quit".to_string()
    },
    Quit,
  ));

  let mut view_items = vec![
    repo_item("Quick Open...", QuickOpen, ctx),
    MenuItem::separator(),
    repo_item("Changes", ShowChanges, ctx),
    repo_item("History", ShowHistory, ctx),
    repo_item("Toggle Diff Mode", ToggleDiffLayout, ctx),
    MenuItem::separator(),
    MenuItem::action("Color Theme...", ColorTheme),
    MenuItem::separator(),
    MenuItem::action("Zoom In", ZoomIn),
    MenuItem::action("Zoom Out", ZoomOut),
    MenuItem::action("Reset Zoom", ZoomReset),
  ];
  if cfg!(debug_assertions) {
    view_items.push(MenuItem::separator());
    view_items.push(MenuItem::action("Inspect Element", InspectElement));
  }

  vec![
    menu("DeathPush", app_items),
    menu(
      "File",
      vec![
        MenuItem::action("New Window", NewWindow),
        MenuItem::action("Open Repository...", OpenRepository),
        MenuItem::action("Clone Repository...", CloneRepository),
        MenuItem::separator(),
        MenuItem::action(if windows { "Close" } else { "Close Window" }, CloseWindow),
      ],
    ),
    menu(
      "Edit",
      vec![
        MenuItem::os_action("Undo", Undo, OsAction::Undo),
        MenuItem::os_action("Redo", Redo, OsAction::Redo),
        MenuItem::separator(),
        MenuItem::os_action("Cut", Cut, OsAction::Cut),
        MenuItem::os_action("Copy", Copy, OsAction::Copy),
        MenuItem::os_action("Paste", Paste, OsAction::Paste),
        MenuItem::separator(),
        MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
      ],
    ),
    menu("View", view_items),
    menu(
      "Git",
      vec![
        repo_item("Pull", GitPull, ctx),
        repo_item("Push", GitPush, ctx),
        repo_item("Fetch", GitFetch, ctx),
        MenuItem::separator(),
        repo_item("Stage All", GitStageAll, ctx),
        repo_item("Unstage All", GitUnstageAll, ctx),
        MenuItem::separator(),
        repo_item("Stash...", GitStash, ctx),
        repo_item("Stash Pop", GitStashPop, ctx),
        MenuItem::separator(),
        repo_item("Undo Last Commit", GitUndoCommit, ctx),
      ],
    ),
    menu(
      "Terminal",
      vec![
        repo_item("New Terminal", NewTerminal, ctx),
        repo_item("Kill Terminal", KillTerminal, ctx),
        repo_item("Toggle Terminal", ToggleTerminal, ctx),
      ],
    ),
    menu(
      "Window",
      vec![
        MenuItem::action("Minimize", Minimize),
        MenuItem::action(if mac { "Zoom" } else { "Maximize" }, Maximize),
        MenuItem::separator(),
        MenuItem::action(if windows { "Close" } else { "Close Window" }, CloseWindow),
      ],
    ),
    menu("Help", vec![MenuItem::action("Open Source Licenses", OpenLicenses)]),
  ]
}

/// One row of the Linux dropdown, in spec order. `separator_before` starts a new group.
#[allow(dead_code)]
pub struct LinuxRow {
  pub label: &'static str,
  pub shortcut: Option<&'static str>,
  pub action: Box<dyn Action>,
  pub disabled: bool,
  pub separator_before: bool,
}

#[allow(dead_code)]
pub fn linux_rows(ctx: &MenuContext) -> Vec<LinuxRow> {
  let repo = !ctx.repo_open;
  let row = |label, shortcut, action: Box<dyn Action>, disabled, separator_before| LinuxRow {
    label,
    shortcut,
    action,
    disabled,
    separator_before,
  };
  vec![
    row("New Window", Some("Ctrl+N"), Box::new(NewWindow), false, false),
    row(
      "Open Repository...",
      Some("Ctrl+O"),
      Box::new(OpenRepository),
      false,
      false,
    ),
    row("Clone Repository...", None, Box::new(CloneRepository), false, false),
    row("Changes", Some("Ctrl+1"), Box::new(ShowChanges), repo, true),
    row("History", Some("Ctrl+Shift+2"), Box::new(ShowHistory), repo, false),
    row(
      "Toggle Diff Mode",
      Some("Ctrl+Shift+P"),
      Box::new(ToggleDiffLayout),
      repo,
      false,
    ),
    row("Color Theme...", None, Box::new(ColorTheme), false, true),
    row("Zoom In", Some("Ctrl+="), Box::new(ZoomIn), false, false),
    row("Zoom Out", Some("Ctrl+-"), Box::new(ZoomOut), false, false),
    row("Reset Zoom", Some("Ctrl+0"), Box::new(ZoomReset), false, false),
    row("Pull", None, Box::new(GitPull), repo, true),
    row("Push", None, Box::new(GitPush), repo, false),
    row("Fetch", None, Box::new(GitFetch), repo, false),
    row("Stage All", None, Box::new(GitStageAll), repo, false),
    row("Unstage All", None, Box::new(GitUnstageAll), repo, false),
    row("Stash...", None, Box::new(GitStash), repo, false),
    row("Stash Pop", None, Box::new(GitStashPop), repo, false),
    row("Undo Last Commit", None, Box::new(GitUndoCommit), repo, false),
    row("New Terminal", Some("Ctrl+Shift+J"), Box::new(NewTerminal), repo, true),
    row("Kill Terminal", None, Box::new(KillTerminal), repo, false),
    row("Toggle Terminal", Some("Ctrl+J"), Box::new(ToggleTerminal), repo, false),
    row("Settings...", Some("Ctrl+,"), Box::new(ShowSettings), false, true),
    row("Quit", None, Box::new(Quit), false, false),
  ]
}

/// Rebuild the native menu bar from the global state.
pub fn refresh_menus(cx: &mut App) {
  let ctx = cx.default_global::<MenuState>().0;
  cx.set_menus(build_menus(&ctx));
}

#[allow(dead_code)]
pub fn set_menu_context(ctx: MenuContext, cx: &mut App) {
  if cx.default_global::<MenuState>().0 != ctx {
    cx.default_global::<MenuState>().0 = ctx;
    refresh_menus(cx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  fn names(menu: &Menu) -> Vec<String> {
    menu
      .items
      .iter()
      .filter_map(|item| match item {
        MenuItem::Action { name, .. } => Some(name.to_string()),
        _ => None,
      })
      .collect()
  }

  fn disabled(menu: &Menu, label: &str) -> bool {
    menu
      .items
      .iter()
      .find_map(|item| match item {
        MenuItem::Action { name, disabled, .. } if name.as_ref() == label => Some(*disabled),
        _ => None,
      })
      .unwrap_or_else(|| panic!("no item {label}"))
  }

  #[test]
  fn top_level_order_matches_the_spec() {
    let menus = build_menus(&MenuContext::default());
    let titles: Vec<&str> = menus.iter().map(|m| m.name.as_ref()).collect();
    assert_eq!(
      titles,
      vec!["DeathPush", "File", "Edit", "View", "Git", "Terminal", "Window", "Help"]
    );
  }

  #[test]
  fn repo_only_items_follow_repo_open() {
    let closed = build_menus(&MenuContext {
      repo_open: false,
      cli_installed: false,
    });
    assert!(disabled(&closed[4], "Pull"));
    assert!(disabled(&closed[3], "Changes"));
    assert!(!disabled(&closed[3], "Zoom In"));
    assert!(disabled(&closed[5], "Toggle Terminal"));
    let open = build_menus(&MenuContext {
      repo_open: true,
      cli_installed: false,
    });
    assert!(!disabled(&open[4], "Pull"));
    assert!(!disabled(&open[3], "Changes"));
  }

  #[test]
  fn cli_item_flips_between_install_and_uninstall() {
    let not_installed = build_menus(&MenuContext::default());
    let installed = build_menus(&MenuContext {
      repo_open: false,
      cli_installed: true,
    });
    if cfg!(target_os = "linux") {
      assert!(!names(&not_installed[0]).iter().any(|n| n.contains("Command Line")));
    } else {
      assert!(names(&not_installed[0]).contains(&"Install Command Line Tool...".to_string()));
      assert!(names(&installed[0]).contains(&"Uninstall Command Line Tool...".to_string()));
    }
  }

  #[test]
  fn linux_rows_match_the_spec_table() {
    let rows = linux_rows(&MenuContext::default());
    assert_eq!(rows.len(), 23);
    assert_eq!(rows[0].label, "New Window");
    assert_eq!(rows[22].label, "Quit");
    assert!(rows.iter().find(|r| r.label == "Changes").unwrap().disabled);
    assert!(!rows.iter().find(|r| r.label == "Settings...").unwrap().disabled);
    assert_eq!(rows.iter().filter(|r| r.separator_before).count(), 5);
  }
}
