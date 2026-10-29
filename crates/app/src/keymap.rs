use std::rc::Rc;

use gpui_kit::*;

use crate::actions::*;

#[allow(dead_code)]
pub const PRIMARY: &str = if cfg!(target_os = "macos") { "cmd" } else { "ctrl" };

/// Key contexts the shell sets on its regions.
pub const CONTEXT_APP: &str = "DeathPush";
pub const CONTEXT_WELCOME: &str = "Welcome";
pub const CONTEXT_WELCOME_LIST: &str = "WelcomeList";
pub const CONTEXT_WELCOME_LIST_INPUT: &str = "WelcomeList > Input";
pub const CONTEXT_REPOSITORY: &str = "Repository";
pub const CONTEXT_REPOSITORY_KEYS: &str = "Repository && !Terminal && !Settings";
pub const CONTEXT_SETTINGS: &str = "Settings";
pub const CONTEXT_DIALOG: &str = "Dialog";
#[allow(dead_code)]
pub const CONTEXT_CHANGES: &str = "Changes";
pub const CONTEXT_CHANGES_INPUT: &str = "Changes > Input";
pub const CONTEXT_DIFF: &str = "Diff";
pub const CONTEXT_BRANCH_LIST: &str = "BranchList";
pub const CONTEXT_BRANCH_LIST_INPUT: &str = "BranchList > Input";
pub const CONTEXT_BRANCH_PICKER: &str = "BranchPicker";
pub const CONTEXT_BRANCH_PICKER_INPUT: &str = "BranchPicker > Input";
pub const CONTEXT_EXPLORER: &str = "Explorer";
pub const CONTEXT_EXPLORER_INPUT: &str = "Explorer > Input";
pub const CONTEXT_EXPLORER_KEYS: &str = "Explorer && !Input";
pub const CONTEXT_TERMINAL: &str = "Terminal";
pub const CONTEXT_TERMINAL_FIND: &str = "TerminalFind";
pub const CONTEXT_TERMINAL_FIND_INPUT: &str = "TerminalFind > Input";

/// (keystrokes, action name, context). Pure so it can be tested per platform.
pub fn binding_table(mac: bool) -> Vec<(String, &'static str, Option<&'static str>)> {
  let m = if mac { "cmd" } else { "ctrl" };
  let mut rows = vec![
    (format!("{m}-n"), "NewWindow", Some(CONTEXT_APP)),
    (format!("{m}-o"), "OpenRepository", Some(CONTEXT_APP)),
    (format!("{m}-,"), "ShowSettings", Some(CONTEXT_APP)),
    (format!("{m}-p"), "QuickOpen", Some(CONTEXT_APP)),
    (format!("{m}-p"), "QuickOpen", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-1"), "ShowChanges", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-2"), "ShowExplorer", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-3"), "FocusTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-2"), "ShowHistory", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-p"), "ToggleDiffLayout", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-j"), "ToggleTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-j"), "NewTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-t"), "NewTerminal", Some(CONTEXT_TERMINAL)),
    (format!("{m}-d"), "SplitTerminalHorizontal", Some(CONTEXT_TERMINAL)),
    (format!("{m}-shift-d"), "SplitTerminalVertical", Some(CONTEXT_TERMINAL)),
    (format!("{m}-w"), "KillTerminalPane", Some(CONTEXT_TERMINAL)),
    (format!("{m}-f"), "FindInTerminal", Some(CONTEXT_TERMINAL)),
    (format!("{m}-f"), "FindInTerminal", Some(CONTEXT_TERMINAL_FIND)),
    (
      "escape".to_string(),
      "CloseTerminalFind",
      Some(CONTEXT_TERMINAL_FIND_INPUT),
    ),
    (format!("alt-{m}-1"), "ActivateTerminalGroup1", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-2"), "ActivateTerminalGroup2", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-3"), "ActivateTerminalGroup3", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-4"), "ActivateTerminalGroup4", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-5"), "ActivateTerminalGroup5", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-6"), "ActivateTerminalGroup6", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-7"), "ActivateTerminalGroup7", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-8"), "ActivateTerminalGroup8", Some(CONTEXT_REPOSITORY)),
    (format!("alt-{m}-9"), "ActivateTerminalGroup9", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-g"), "ReloadSession", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-s"), "SwallowSave", Some(CONTEXT_REPOSITORY)),
    ("escape".to_string(), "ClearSelection", Some(CONTEXT_REPOSITORY_KEYS)),
    ("f7".to_string(), "NextDiffChange", Some(CONTEXT_DIFF)),
    ("shift-f7".to_string(), "PreviousDiffChange", Some(CONTEXT_DIFF)),
    (format!("{m}-enter"), "CommitFromBox", Some(CONTEXT_CHANGES_INPUT)),
    ("enter".to_string(), "Confirm", Some(CONTEXT_BRANCH_LIST_INPUT)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_BRANCH_LIST)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_BRANCH_LIST_INPUT)),
    ("enter".to_string(), "Confirm", Some(CONTEXT_BRANCH_PICKER_INPUT)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_BRANCH_PICKER)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_BRANCH_PICKER_INPUT)),
    (format!("{m}-1"), "FocusRecentFilter", Some(CONTEXT_WELCOME)),
    (format!("{m}-2"), "FocusWorkspaceFilter", Some(CONTEXT_WELCOME)),
    (format!("{m}-="), "ZoomIn", Some(CONTEXT_APP)),
    (format!("{m}-shift-="), "ZoomIn", Some(CONTEXT_APP)),
    (format!("{m}--"), "ZoomOut", Some(CONTEXT_APP)),
    (format!("{m}-0"), "ZoomReset", Some(CONTEXT_APP)),
    (format!("{m}-k {m}-t"), "ColorTheme", Some(CONTEXT_APP)),
    (format!("{m}-m"), "Minimize", Some(CONTEXT_APP)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_DIALOG)),
    ("enter".to_string(), "Confirm", Some(CONTEXT_DIALOG)),
    ("up".to_string(), "WelcomeListUp", Some(CONTEXT_WELCOME_LIST_INPUT)),
    ("down".to_string(), "WelcomeListDown", Some(CONTEXT_WELCOME_LIST_INPUT)),
    (
      "enter".to_string(),
      "WelcomeListConfirm",
      Some(CONTEXT_WELCOME_LIST_INPUT),
    ),
    (
      "escape".to_string(),
      "WelcomeListEscape",
      Some(CONTEXT_WELCOME_LIST_INPUT),
    ),
    ("f2".to_string(), "ExplorerRename", Some(CONTEXT_EXPLORER_KEYS)),
    ("delete".to_string(), "ExplorerDelete", Some(CONTEXT_EXPLORER_KEYS)),
    (format!("{m}-backspace"), "ExplorerDelete", Some(CONTEXT_EXPLORER_KEYS)),
    (format!("{m}-c"), "ExplorerCopy", Some(CONTEXT_EXPLORER_KEYS)),
    (format!("{m}-x"), "ExplorerCut", Some(CONTEXT_EXPLORER_KEYS)),
    (format!("{m}-v"), "ExplorerPaste", Some(CONTEXT_EXPLORER_KEYS)),
    ("escape".to_string(), "Cancel", Some(CONTEXT_EXPLORER_INPUT)),
  ];
  if mac {
    rows.push(("cmd-w".to_string(), "CloseWindow", Some(CONTEXT_APP)));
    rows.push(("cmd-q".to_string(), "Quit", None));
    rows.push(("cmd-h".to_string(), "Hide", None));
    rows.push(("alt-cmd-h".to_string(), "HideOthers", None));
  } else {
    rows.push(("alt-f4".to_string(), "CloseWindow", Some(CONTEXT_APP)));
  }
  if cfg!(debug_assertions) {
    rows.push((format!("{m}-shift-i"), "InspectElement", Some(CONTEXT_APP)));
  }
  rows
}

fn binding_for(keys: &str, name: &str, context: Option<&str>, cx: &App) -> KeyBinding {
  let context = context.map(|source| Rc::new(KeyBindingContextPredicate::parse(source).expect("valid key context")));
  let bind = |action: Box<dyn Action>| {
    KeyBinding::load(keys, action, context.clone(), false, None, cx).expect("valid keystrokes")
  };
  match name {
    "NewWindow" => bind(Box::new(NewWindow)),
    "OpenRepository" => bind(Box::new(OpenRepository)),
    "ShowSettings" => bind(Box::new(ShowSettings)),
    "QuickOpen" => bind(Box::new(QuickOpen)),
    "ShowChanges" => bind(Box::new(ShowChanges)),
    "ShowExplorer" => bind(Box::new(ShowExplorer)),
    "FocusTerminal" => bind(Box::new(FocusTerminal)),
    "ShowHistory" => bind(Box::new(ShowHistory)),
    "ToggleDiffLayout" => bind(Box::new(ToggleDiffLayout)),
    "ToggleTerminal" => bind(Box::new(ToggleTerminal)),
    "NewTerminal" => bind(Box::new(NewTerminal)),
    "SplitTerminalHorizontal" => bind(Box::new(SplitTerminalHorizontal)),
    "SplitTerminalVertical" => bind(Box::new(SplitTerminalVertical)),
    "KillTerminalPane" => bind(Box::new(KillTerminalPane)),
    "FindInTerminal" => bind(Box::new(FindInTerminal)),
    "CloseTerminalFind" => bind(Box::new(CloseTerminalFind)),
    "ActivateTerminalGroup1" => bind(Box::new(ActivateTerminalGroup1)),
    "ActivateTerminalGroup2" => bind(Box::new(ActivateTerminalGroup2)),
    "ActivateTerminalGroup3" => bind(Box::new(ActivateTerminalGroup3)),
    "ActivateTerminalGroup4" => bind(Box::new(ActivateTerminalGroup4)),
    "ActivateTerminalGroup5" => bind(Box::new(ActivateTerminalGroup5)),
    "ActivateTerminalGroup6" => bind(Box::new(ActivateTerminalGroup6)),
    "ActivateTerminalGroup7" => bind(Box::new(ActivateTerminalGroup7)),
    "ActivateTerminalGroup8" => bind(Box::new(ActivateTerminalGroup8)),
    "ActivateTerminalGroup9" => bind(Box::new(ActivateTerminalGroup9)),
    "ReloadSession" => bind(Box::new(ReloadSession)),
    "SwallowSave" => bind(Box::new(SwallowSave)),
    "ClearSelection" => bind(Box::new(ClearSelection)),
    "NextDiffChange" => bind(Box::new(NextDiffChange)),
    "PreviousDiffChange" => bind(Box::new(PreviousDiffChange)),
    "CommitFromBox" => bind(Box::new(CommitFromBox)),
    "FocusRecentFilter" => bind(Box::new(FocusRecentFilter)),
    "FocusWorkspaceFilter" => bind(Box::new(FocusWorkspaceFilter)),
    "WelcomeListUp" => bind(Box::new(WelcomeListUp)),
    "WelcomeListDown" => bind(Box::new(WelcomeListDown)),
    "WelcomeListConfirm" => bind(Box::new(WelcomeListConfirm)),
    "WelcomeListEscape" => bind(Box::new(WelcomeListEscape)),
    "ZoomIn" => bind(Box::new(ZoomIn)),
    "ZoomOut" => bind(Box::new(ZoomOut)),
    "ZoomReset" => bind(Box::new(ZoomReset)),
    "ColorTheme" => bind(Box::new(ColorTheme)),
    "Minimize" => bind(Box::new(Minimize)),
    "Cancel" => bind(Box::new(Cancel)),
    "Confirm" => bind(Box::new(Confirm)),
    "CloseWindow" => bind(Box::new(CloseWindow)),
    "Quit" => bind(Box::new(Quit)),
    "Hide" => bind(Box::new(Hide)),
    "HideOthers" => bind(Box::new(HideOthers)),
    "InspectElement" => bind(Box::new(InspectElement)),
    "ExplorerRename" => bind(Box::new(ExplorerRename)),
    "ExplorerDelete" => bind(Box::new(ExplorerDelete)),
    "ExplorerCut" => bind(Box::new(ExplorerCut)),
    "ExplorerCopy" => bind(Box::new(ExplorerCopy)),
    "ExplorerPaste" => bind(Box::new(ExplorerPaste)),
    other => unreachable!("unknown action {other}"),
  }
}

pub fn bindings(cx: &App) -> Vec<KeyBinding> {
  binding_table(cfg!(target_os = "macos"))
    .into_iter()
    .map(|(keys, name, context)| binding_for(&keys, name, context, cx))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;
  use core::prelude::v1::test;

  #[test]
  fn mac_uses_cmd_and_others_use_ctrl() {
    let mac = binding_table(true);
    let other = binding_table(false);
    assert!(
      mac
        .iter()
        .any(|(keys, name, _)| keys == "cmd-1" && *name == "ShowChanges")
    );
    assert!(
      other
        .iter()
        .any(|(keys, name, _)| keys == "ctrl-1" && *name == "ShowChanges")
    );
    assert!(mac.iter().any(|(keys, _, _)| keys == "cmd-q"));
    assert!(!other.iter().any(|(_, name, _)| *name == "Quit"));
    assert!(
      other
        .iter()
        .any(|(keys, name, _)| keys == "alt-f4" && *name == "CloseWindow")
    );
  }

  #[test]
  fn welcome_and_repository_share_the_number_keys_in_different_contexts() {
    let rows = binding_table(true);
    let one: Vec<_> = rows.iter().filter(|(keys, _, _)| keys == "cmd-1").collect();
    assert_eq!(one.len(), 2);
    assert!(
      one
        .iter()
        .any(|(_, name, ctx)| *name == "ShowChanges" && *ctx == Some(CONTEXT_REPOSITORY))
    );
    assert!(
      one
        .iter()
        .any(|(_, name, ctx)| *name == "FocusRecentFilter" && *ctx == Some(CONTEXT_WELCOME))
    );
  }

  #[gpui_kit::test]
  fn every_table_row_builds_a_binding(cx: &mut gpui_kit::TestAppContext) {
    let count = binding_table(cfg!(target_os = "macos")).len();
    cx.update(|cx| assert_eq!(bindings(cx).len(), count));
  }

  #[test]
  fn escape_binds_clear_selection_only_in_repository() {
    let rows = binding_table(true);
    let clear: Vec<_> = rows.iter().filter(|(_, name, _)| *name == "ClearSelection").collect();
    assert_eq!(clear.len(), 1);
    assert_eq!(clear[0].0, "escape");
    assert_eq!(clear[0].2, Some("Repository && !Terminal && !Settings"));
    assert_eq!(CONTEXT_SETTINGS, "Settings");
  }

  #[test]
  fn scm_and_diff_keys_bind_in_their_contexts() {
    for (mac, primary) in [(true, "cmd"), (false, "ctrl")] {
      let rows = binding_table(mac);
      assert!(
        rows.iter().any(|(keys, name, ctx)| {
          keys == &format!("{primary}-enter") && *name == "CommitFromBox" && *ctx == Some(CONTEXT_CHANGES_INPUT)
        }),
        "{primary}-enter -> CommitFromBox in Changes > Input"
      );
      for (keys, name) in [("f7", "NextDiffChange"), ("shift-f7", "PreviousDiffChange")] {
        assert!(
          rows
            .iter()
            .any(|(row_keys, row_name, ctx)| row_keys == keys && *row_name == name && *ctx == Some(CONTEXT_DIFF)),
          "{keys} -> {name} in Diff"
        );
      }
    }
  }

  #[test]
  fn explorer_keys_bind_in_explorer_context() {
    for (mac, primary) in [(true, "cmd"), (false, "ctrl")] {
      let rows = binding_table(mac);
      let context = Some(CONTEXT_EXPLORER_KEYS);
      let expected = [
        ("f2".to_string(), "ExplorerRename"),
        ("delete".to_string(), "ExplorerDelete"),
        (format!("{primary}-backspace"), "ExplorerDelete"),
        (format!("{primary}-c"), "ExplorerCopy"),
        (format!("{primary}-x"), "ExplorerCut"),
        (format!("{primary}-v"), "ExplorerPaste"),
      ];
      for (key, name) in expected {
        assert!(
          rows
            .iter()
            .any(|(keys, action, ctx)| keys == &key && *action == name && *ctx == context),
          "{key} -> {name} in Explorer && !Input"
        );
      }
      assert!(
        rows.iter().any(|(keys, action, ctx)| {
          keys == "escape" && *action == "Cancel" && *ctx == Some(CONTEXT_EXPLORER_INPUT)
        }),
        "escape -> Cancel in Explorer > Input"
      );
    }
  }

  #[test]
  fn quick_open_binds_in_app_and_repository() {
    for (mac, primary) in [(true, "cmd"), (false, "ctrl")] {
      let rows = binding_table(mac);
      let key = format!("{primary}-p");
      assert!(
        rows
          .iter()
          .any(|(keys, name, ctx)| keys == &key && *name == "QuickOpen" && *ctx == Some(CONTEXT_APP)),
        "{key} -> QuickOpen in DeathPush"
      );
      assert!(
        rows
          .iter()
          .any(|(keys, name, ctx)| { keys == &key && *name == "QuickOpen" && *ctx == Some(CONTEXT_REPOSITORY) }),
        "{key} -> QuickOpen in Repository"
      );
    }
  }

  #[test]
  fn branch_picker_keys_bind_in_picker_context() {
    let rows = binding_table(true);
    assert!(
      rows.iter().any(|(keys, name, ctx)| {
        keys == "enter" && *name == "Confirm" && *ctx == Some(CONTEXT_BRANCH_PICKER_INPUT)
      }),
      "enter -> Confirm in BranchPicker > Input"
    );
    assert!(
      rows
        .iter()
        .any(|(keys, name, ctx)| keys == "escape" && *name == "Cancel" && *ctx == Some(CONTEXT_BRANCH_PICKER)),
      "escape -> Cancel in BranchPicker"
    );
    assert!(
      rows.iter().any(|(keys, name, ctx)| {
        keys == "escape" && *name == "Cancel" && *ctx == Some(CONTEXT_BRANCH_PICKER_INPUT)
      }),
      "escape -> Cancel in BranchPicker > Input"
    );
  }

  #[test]
  fn color_theme_chord_binds_in_app_context() {
    for (mac, primary) in [(true, "cmd"), (false, "ctrl")] {
      let rows = binding_table(mac);
      let key = format!("{primary}-k {primary}-t");
      assert!(
        rows
          .iter()
          .any(|(keys, name, ctx)| keys == &key && *name == "ColorTheme" && *ctx == Some(CONTEXT_APP)),
        "{key} -> ColorTheme in DeathPush"
      );
    }
  }

  #[test]
  fn welcome_list_keys_bind_inside_the_input_context() {
    let rows = binding_table(true);
    let context = Some(CONTEXT_WELCOME_LIST_INPUT);
    for (key, name) in [
      ("up", "WelcomeListUp"),
      ("down", "WelcomeListDown"),
      ("enter", "WelcomeListConfirm"),
      ("escape", "WelcomeListEscape"),
    ] {
      assert!(
        rows
          .iter()
          .any(|(keys, action, ctx)| keys == key && *action == name && *ctx == context),
        "{key} -> {name} in WelcomeList > Input"
      );
    }
  }

  #[test]
  fn terminal_context_and_group_chords() {
    for (mac, primary) in [(true, "cmd"), (false, "ctrl")] {
      let rows = binding_table(mac);
      let term = Some(CONTEXT_TERMINAL);
      let expected = [
        (format!("{primary}-t"), "NewTerminal"),
        (format!("{primary}-d"), "SplitTerminalHorizontal"),
        (format!("{primary}-shift-d"), "SplitTerminalVertical"),
        (format!("{primary}-w"), "KillTerminalPane"),
        (format!("{primary}-f"), "FindInTerminal"),
      ];
      for (key, name) in expected {
        assert!(
          rows
            .iter()
            .any(|(keys, action, ctx)| keys == &key && *action == name && *ctx == term),
          "{key} -> {name} in Terminal"
        );
      }
      for n in 1..=9 {
        let key = format!("alt-{primary}-{n}");
        let name = format!("ActivateTerminalGroup{n}");
        assert!(
          rows
            .iter()
            .any(|(keys, action, ctx)| keys == &key && *action == name && *ctx == Some(CONTEXT_REPOSITORY)),
          "{key} -> {name} in Repository"
        );
      }
    }
  }

  #[test]
  fn find_widget_keys_bind_inside_the_widget() {
    for mac in [true, false] {
      let rows = binding_table(mac);
      assert!(rows.iter().any(|(keys, action, ctx)| keys == "escape"
        && *action == "CloseTerminalFind"
        && *ctx == Some(CONTEXT_TERMINAL_FIND_INPUT)));
      let find = if mac { "cmd-f" } else { "ctrl-f" };
      assert!(
        rows.iter().any(|(keys, action, ctx)| keys == find
          && *action == "FindInTerminal"
          && *ctx == Some(CONTEXT_TERMINAL_FIND))
      );
    }
  }
}
