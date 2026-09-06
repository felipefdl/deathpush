use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use crate::config::settings::WorkspaceEntry;
use crate::types::ProjectInfo;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct WorkspaceNode {
  pub name: String,
  /// Stable identity for expansion state: the directory for a root, `<parent key>/<name>` below it.
  pub key: String,
  pub children: BTreeMap<String, WorkspaceNode>,
  pub projects: Vec<ProjectInfo>,
}

fn normalize(directory: &str) -> String {
  directory.trim_end_matches(['/', '\\']).to_string()
}

fn slash_path(path: &str) -> String {
  path.replace('\\', "/")
}

fn insert_under(root: &mut WorkspaceNode, root_directory: &str, projects: &[ProjectInfo]) {
  let root_slash = slash_path(root_directory);
  for project in projects {
    let path_slash = slash_path(&project.path);
    let Some(relative) = path_slash.strip_prefix(&format!("{root_slash}/")) else {
      continue;
    };
    let parts: Vec<&str> = relative.split(['/', '\\']).collect();
    let mut current = &mut *root;
    for part in &parts[..parts.len().saturating_sub(1)] {
      let key = format!("{}/{part}", current.key);
      current = current
        .children
        .entry(part.to_string())
        .or_insert_with(|| WorkspaceNode {
          name: part.to_string(),
          key,
          ..Default::default()
        });
    }
    current.projects.push(project.clone());
  }
}

/// One node per workspace directory; deeper directories become nested nodes when the scan depth is above 1.
pub fn build_tree(projects: &[ProjectInfo], workspaces: &[WorkspaceEntry]) -> WorkspaceNode {
  let mut root = WorkspaceNode::default();
  let mut sorted: Vec<(String, String, u32)> = workspaces
    .iter()
    .map(|ws| {
      let dir = normalize(&ws.directory);
      let slash = slash_path(&dir);
      (dir, slash, ws.scan_depth)
    })
    .collect();
  sorted.sort_by_key(|entry| std::cmp::Reverse(entry.0.len()));
  let mut by_workspace: BTreeMap<String, Vec<ProjectInfo>> =
    sorted.iter().map(|(dir, _, _)| (dir.clone(), Vec::new())).collect();
  for project in projects {
    let path_slash = slash_path(&project.path);
    if let Some((dir, _, _)) = sorted
      .iter()
      .find(|(_, dir_slash, _)| path_slash.starts_with(&format!("{dir_slash}/")))
    {
      by_workspace.get_mut(dir).unwrap().push(project.clone());
    }
  }
  for (dir, slash, depth) in &sorted {
    let name = Path::new(slash)
      .file_name()
      .map(|n| n.to_string_lossy().into_owned())
      .filter(|n| !n.is_empty())
      .unwrap_or_else(|| dir.clone());
    let mut node = WorkspaceNode {
      name,
      key: dir.clone(),
      ..Default::default()
    };
    let members = &by_workspace[dir];
    if *depth > 1 {
      insert_under(&mut node, dir, members);
    } else {
      node.projects = members.clone();
    }
    root.children.insert(dir.clone(), node);
  }
  root
}

/// Tree when more than one workspace or any depth above 1, the filter is empty, and no keyboard highlight is active.
pub fn is_tree_layout(workspaces: &[WorkspaceEntry], filter: &str, keyboard_highlight: bool) -> bool {
  let nested = workspaces.len() > 1 || workspaces.iter().any(|ws| ws.scan_depth > 1);
  nested && filter.trim().is_empty() && !keyboard_highlight
}

#[derive(Debug, Clone, PartialEq)]
pub enum WorkspaceRow {
  Folder {
    key: String,
    name: String,
    depth: usize,
    expanded: bool,
  },
  Project {
    name: String,
    path: String,
    depth: usize,
  },
}

fn push_node(node: &WorkspaceNode, depth: usize, expanded: &HashSet<String>, rows: &mut Vec<WorkspaceRow>) {
  let mut children: Vec<&WorkspaceNode> = node.children.values().collect();
  children.sort_by_key(|child| child.name.to_lowercase());
  for child in children {
    let is_expanded = expanded.contains(&child.key);
    rows.push(WorkspaceRow::Folder {
      key: child.key.clone(),
      name: child.name.clone(),
      depth,
      expanded: is_expanded,
    });
    if is_expanded {
      push_node(child, depth + 1, expanded, rows);
    }
  }
  let mut projects = node.projects.clone();
  projects.sort_by_key(|project| project.name.to_lowercase());
  for project in projects {
    rows.push(WorkspaceRow::Project {
      name: project.name,
      path: project.path,
      depth,
    });
  }
}

/// Folders first, then projects, recursing into expanded folders only.
pub fn tree_rows(tree: &WorkspaceNode, expanded: &HashSet<String>) -> Vec<WorkspaceRow> {
  let mut rows = Vec::new();
  push_node(tree, 0, expanded, &mut rows);
  rows
}

/// Every project as a depth-0 row, filtered by name or path, sorted by name.
pub fn flat_rows(projects: &[ProjectInfo], filter: &str) -> Vec<WorkspaceRow> {
  let needle = filter.trim().to_lowercase();
  let mut matching: Vec<&ProjectInfo> = projects
    .iter()
    .filter(|project| {
      needle.is_empty()
        || project.name.to_lowercase().contains(&needle)
        || project.path.to_lowercase().contains(&needle)
    })
    .collect();
  matching.sort_by_key(|project| project.name.to_lowercase());
  matching
    .into_iter()
    .map(|project| WorkspaceRow::Project {
      name: project.name.clone(),
      path: project.path.clone(),
      depth: 0,
    })
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  fn project(path: &str) -> ProjectInfo {
    ProjectInfo {
      path: path.into(),
      name: path.rsplit(['/', '\\']).next().unwrap().into(),
    }
  }

  fn workspace(directory: &str, scan_depth: u32) -> WorkspaceEntry {
    WorkspaceEntry {
      directory: directory.into(),
      scan_depth,
    }
  }

  #[test]
  fn depth_one_workspace_lists_projects_flat_under_the_root_node() {
    let tree = build_tree(&[project("/w/a"), project("/w/b")], &[workspace("/w/", 1)]);
    let node = &tree.children["/w"];
    assert_eq!(node.name, "w");
    assert_eq!(node.projects.len(), 2);
    assert!(node.children.is_empty());
  }

  #[test]
  fn deeper_workspace_nests_intermediate_folders() {
    let tree = build_tree(&[project("/w/group/a"), project("/w/b")], &[workspace("/w", 3)]);
    let node = &tree.children["/w"];
    assert_eq!(node.projects.len(), 1);
    let group = &node.children["group"];
    assert_eq!(group.key, "/w/group");
    assert_eq!(group.projects[0].name, "a");
  }

  #[test]
  fn longest_workspace_prefix_wins() {
    let tree = build_tree(
      &[project("/w/inner/x")],
      &[workspace("/w", 2), workspace("/w/inner", 1)],
    );
    assert!(tree.children["/w/inner"].projects.iter().any(|p| p.name == "x"));
    assert!(tree.children["/w"].projects.is_empty());
  }

  #[test]
  fn tree_rows_follow_expansion() {
    let tree = build_tree(&[project("/w/group/a"), project("/w/b")], &[workspace("/w", 3)]);
    let collapsed = tree_rows(&tree, &HashSet::new());
    assert_eq!(collapsed.len(), 1);
    let mut expanded = HashSet::new();
    expanded.insert("/w".to_string());
    let rows = tree_rows(&tree, &expanded);
    assert!(matches!(&rows[1], WorkspaceRow::Folder { name, depth: 1, expanded: false, .. } if name == "group"));
    assert!(matches!(&rows[2], WorkspaceRow::Project { name, depth: 1, .. } if name == "b"));
    expanded.insert("/w/group".to_string());
    let rows = tree_rows(&tree, &expanded);
    assert!(matches!(&rows[2], WorkspaceRow::Project { name, depth: 2, .. } if name == "a"));
  }

  #[test]
  fn windows_paths_nest_the_same_as_posix() {
    let tree = build_tree(
      &[project(r"C:\work\group\a"), project(r"C:\work\b")],
      &[workspace(r"C:\work", 3)],
    );
    let node = &tree.children[r"C:\work"];
    assert_eq!(node.name, "work");
    assert_eq!(node.projects.len(), 1);
    assert_eq!(node.projects[0].name, "b");
    let group = &node.children["group"];
    assert_eq!(group.key, r"C:\work/group");
    assert_eq!(group.projects[0].name, "a");
  }

  #[test]
  fn windows_depth_one_lists_projects_flat() {
    let tree = build_tree(
      &[project(r"C:\work\a"), project(r"C:\work\b")],
      &[workspace(r"C:\work\", 1)],
    );
    let node = &tree.children[r"C:\work"];
    assert_eq!(node.name, "work");
    assert_eq!(node.projects.len(), 2);
    assert!(node.children.is_empty());
  }

  #[test]
  fn layout_rule_matches_the_spec() {
    let single = [workspace("/w", 1)];
    let nested = [workspace("/w", 2)];
    let two = [workspace("/a", 1), workspace("/b", 1)];
    assert!(!is_tree_layout(&single, "", false));
    assert!(is_tree_layout(&nested, "", false));
    assert!(is_tree_layout(&two, "", false));
    assert!(!is_tree_layout(&two, "x", false));
    assert!(!is_tree_layout(&two, "", true));
  }

  #[test]
  fn flat_rows_filter_and_sort() {
    let rows = flat_rows(&[project("/w/zeta"), project("/w/Alpha"), project("/other/beta")], "w/");
    let names: Vec<&str> = rows
      .iter()
      .map(|row| match row {
        WorkspaceRow::Project { name, .. } => name.as_str(),
        _ => unreachable!(),
      })
      .collect();
    assert_eq!(names, vec!["Alpha", "zeta"]);
  }
}
