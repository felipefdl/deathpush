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
pub const CONTEXT_DIALOG: &str = "Dialog";

/// (keystrokes, action name, context). Pure so it can be tested per platform.
pub fn binding_table(mac: bool) -> Vec<(String, &'static str, Option<&'static str>)> {
  let m = if mac { "cmd" } else { "ctrl" };
  let mut rows = vec![
    (format!("{m}-n"), "NewWindow", Some(CONTEXT_APP)),
    (format!("{m}-o"), "OpenRepository", Some(CONTEXT_APP)),
    (format!("{m}-,"), "ShowSettings", Some(CONTEXT_APP)),
    (format!("{m}-p"), "QuickOpen", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-1"), "ShowChanges", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-2"), "ShowExplorer", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-3"), "FocusTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-2"), "ShowHistory", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-p"), "ToggleDiffLayout", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-j"), "ToggleTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-j"), "NewTerminal", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-shift-g"), "ReloadSession", Some(CONTEXT_REPOSITORY)),
    (format!("{m}-s"), "SwallowSave", Some(CONTEXT_REPOSITORY)),
    ("escape".to_string(), "ClearSelection", Some(CONTEXT_REPOSITORY)),
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

fn binding_for(keys: &str, name: &str, context: Option<&str>) -> KeyBinding {
  match name {
    "NewWindow" => KeyBinding::new(keys, NewWindow, context),
    "OpenRepository" => KeyBinding::new(keys, OpenRepository, context),
    "ShowSettings" => KeyBinding::new(keys, ShowSettings, context),
    "QuickOpen" => KeyBinding::new(keys, QuickOpen, context),
    "ShowChanges" => KeyBinding::new(keys, ShowChanges, context),
    "ShowExplorer" => KeyBinding::new(keys, ShowExplorer, context),
    "FocusTerminal" => KeyBinding::new(keys, FocusTerminal, context),
    "ShowHistory" => KeyBinding::new(keys, ShowHistory, context),
    "ToggleDiffLayout" => KeyBinding::new(keys, ToggleDiffLayout, context),
    "ToggleTerminal" => KeyBinding::new(keys, ToggleTerminal, context),
    "NewTerminal" => KeyBinding::new(keys, NewTerminal, context),
    "ReloadSession" => KeyBinding::new(keys, ReloadSession, context),
    "SwallowSave" => KeyBinding::new(keys, SwallowSave, context),
    "ClearSelection" => KeyBinding::new(keys, ClearSelection, context),
    "FocusRecentFilter" => KeyBinding::new(keys, FocusRecentFilter, context),
    "FocusWorkspaceFilter" => KeyBinding::new(keys, FocusWorkspaceFilter, context),
    "WelcomeListUp" => KeyBinding::new(keys, WelcomeListUp, context),
    "WelcomeListDown" => KeyBinding::new(keys, WelcomeListDown, context),
    "WelcomeListConfirm" => KeyBinding::new(keys, WelcomeListConfirm, context),
    "WelcomeListEscape" => KeyBinding::new(keys, WelcomeListEscape, context),
    "ZoomIn" => KeyBinding::new(keys, ZoomIn, context),
    "ZoomOut" => KeyBinding::new(keys, ZoomOut, context),
    "ZoomReset" => KeyBinding::new(keys, ZoomReset, context),
    "ColorTheme" => KeyBinding::new(keys, ColorTheme, context),
    "Minimize" => KeyBinding::new(keys, Minimize, context),
    "Cancel" => KeyBinding::new(keys, Cancel, context),
    "Confirm" => KeyBinding::new(keys, Confirm, context),
    "CloseWindow" => KeyBinding::new(keys, CloseWindow, context),
    "Quit" => KeyBinding::new(keys, Quit, context),
    "Hide" => KeyBinding::new(keys, Hide, context),
    "HideOthers" => KeyBinding::new(keys, HideOthers, context),
    "InspectElement" => KeyBinding::new(keys, InspectElement, context),
    other => unreachable!("unknown action {other}"),
  }
}

pub fn bindings() -> Vec<KeyBinding> {
  binding_table(cfg!(target_os = "macos"))
    .into_iter()
    .map(|(keys, name, context)| binding_for(&keys, name, context))
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

  #[test]
  fn every_table_row_builds_a_binding() {
    let count = binding_table(cfg!(target_os = "macos")).len();
    assert_eq!(bindings().len(), count);
  }

  #[test]
  fn escape_binds_clear_selection_only_in_repository() {
    let rows = binding_table(true);
    let clear: Vec<_> = rows.iter().filter(|(_, name, _)| *name == "ClearSelection").collect();
    assert_eq!(clear.len(), 1);
    assert_eq!(clear[0].0, "escape");
    assert_eq!(clear[0].2, Some(CONTEXT_REPOSITORY));
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
}
