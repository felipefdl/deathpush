use std::collections::HashSet;

use deathpush_core::config::recents::Recents;
use deathpush_core::config::settings::WorkspaceEntry;
use deathpush_core::types::ProjectInfo;
use deathpush_core::workspace::{WorkspaceRow, build_tree, flat_rows, is_tree_layout, tree_rows};

/// Which list holds the keyboard highlight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
  Recent,
  Workspace,
}

#[derive(Debug, Default, Clone)]
pub struct Highlight {
  pub pane: Option<Pane>,
  pub index: usize,
}

pub fn recent_indices(recents: &Recents, filter: &str) -> Vec<usize> {
  recents.filter(filter)
}

pub fn workspace_rows(
  projects: &[ProjectInfo],
  workspaces: &[WorkspaceEntry],
  filter: &str,
  expanded: &HashSet<String>,
  keyboard_highlight: bool,
) -> Vec<WorkspaceRow> {
  if is_tree_layout(workspaces, filter, keyboard_highlight) {
    tree_rows(&build_tree(projects, workspaces), expanded)
  } else {
    flat_rows(projects, filter)
  }
}

/// Move the highlight by `delta` within `len` rows, wrapping at neither end.
pub fn step(highlight: &Highlight, pane: Pane, len: usize, delta: isize) -> Highlight {
  if len == 0 {
    return Highlight { pane: None, index: 0 };
  }
  let current = if highlight.pane == Some(pane) {
    highlight.index as isize
  } else {
    -1
  };
  let next = (current + delta).clamp(0, len as isize - 1);
  Highlight {
    pane: Some(pane),
    index: next as usize,
  }
}

pub fn empty_recent_copy(has_any: bool) -> &'static str {
  if has_any {
    "No matching projects"
  } else {
    "No recent projects"
  }
}

pub fn empty_workspace_copy(configured: bool) -> &'static str {
  if configured {
    "No git repositories found"
  } else {
    "No workspace directories configured"
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn step_clamps_and_starts_at_zero() {
    let none = Highlight::default();
    assert_eq!(step(&none, Pane::Recent, 3, 1).index, 0);
    let last = Highlight {
      pane: Some(Pane::Recent),
      index: 2,
    };
    assert_eq!(step(&last, Pane::Recent, 3, 1).index, 2);
    assert_eq!(step(&last, Pane::Recent, 3, -1).index, 1);
    assert_eq!(step(&last, Pane::Workspace, 3, 1).index, 0);
    assert_eq!(step(&last, Pane::Recent, 0, 1).pane, None);
  }

  #[test]
  fn highlight_flattens_the_workspace_tree() {
    let projects = vec![ProjectInfo {
      path: "/w/g/a".into(),
      name: "a".into(),
    }];
    let workspaces = vec![WorkspaceEntry {
      directory: "/w".into(),
      scan_depth: 3,
    }];
    let tree = workspace_rows(&projects, &workspaces, "", &HashSet::new(), false);
    assert!(matches!(tree[0], WorkspaceRow::Folder { .. }));
    let flat = workspace_rows(&projects, &workspaces, "", &HashSet::new(), true);
    assert!(matches!(flat[0], WorkspaceRow::Project { .. }));
  }

  #[test]
  fn empty_copy_matches_the_spec() {
    assert_eq!(empty_recent_copy(false), "No recent projects");
    assert_eq!(empty_recent_copy(true), "No matching projects");
    assert_eq!(empty_workspace_copy(false), "No workspace directories configured");
    assert_eq!(empty_workspace_copy(true), "No git repositories found");
  }
}
