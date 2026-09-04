use std::collections::HashMap;

use crate::types::{CommitEntry, CommitFileEntry};

const FNV_OFFSET: u64 = 0xcbf29ce484222325;
const FNV_PRIME: u64 = 0x100000001b3;

/// Up to two uppercase initials from the author name ("Ana Lima" → "AL", "ana" → "A", "" → "?").
pub fn initials(name: &str) -> String {
  let mut out = String::new();
  for word in name.split_whitespace().take(2) {
    if let Some(ch) = word.chars().next() {
      out.extend(ch.to_uppercase());
    }
  }
  if out.is_empty() { "?".into() } else { out }
}

/// Twelve hues (degrees) the avatar fallback cycles through.
pub const AVATAR_HUES: [f32; 12] = [0., 30., 60., 90., 120., 150., 180., 210., 240., 270., 300., 330.];

fn fnv1a_64(bytes: &[u8]) -> u64 {
  let mut hash = FNV_OFFSET;
  for &byte in bytes {
    hash ^= u64::from(byte);
    hash = hash.wrapping_mul(FNV_PRIME);
  }
  hash
}

/// A stable hue for a name (FNV-1a over the bytes, modulo the hue count).
pub fn avatar_hue(name: &str) -> f32 {
  AVATAR_HUES[(fnv1a_64(name.as_bytes()) % AVATAR_HUES.len() as u64) as usize]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileNode {
  pub name: String,
  pub path: String,
  pub children: Vec<FileNode>,
  pub file: Option<CommitFileEntry>,
}

#[derive(Default)]
struct PartialNode {
  children: HashMap<String, PartialNode>,
  file: Option<CommitFileEntry>,
}

/// Nested tree for the changed-files tree toggle: folders first, case-insensitive, single-child folders are not collapsed.
pub fn changed_files_tree(files: &[CommitFileEntry]) -> Vec<FileNode> {
  let mut root = PartialNode::default();
  for file in files {
    let mut node = &mut root;
    let parts: Vec<&str> = file.path.split('/').filter(|part| !part.is_empty()).collect();
    if parts.is_empty() {
      continue;
    }
    for (index, part) in parts.iter().enumerate() {
      node = node.children.entry((*part).to_string()).or_default();
      if index + 1 == parts.len() {
        node.file = Some(file.clone());
      }
    }
  }
  into_nodes(root, "")
}

fn into_nodes(node: PartialNode, prefix: &str) -> Vec<FileNode> {
  let mut out: Vec<FileNode> = node
    .children
    .into_iter()
    .map(|(name, child)| {
      let path = if prefix.is_empty() {
        name.clone()
      } else {
        format!("{prefix}/{name}")
      };
      let PartialNode { children, file } = child;
      FileNode {
        name,
        path: path.clone(),
        children: into_nodes(PartialNode { children, file: None }, &path),
        file,
      }
    })
    .collect();
  out.sort_by_cached_key(|node| {
    let file_after_folders = node.children.is_empty() && node.file.is_some();
    (file_after_folders, node.name.to_lowercase())
  });
  out
}

fn short_parent(id: &str) -> &str {
  match id.char_indices().nth(7) {
    Some((index, _)) => &id[..index],
    None => id,
  }
}

/// `Merge: {p1}, {p2}` using the first 7 characters of each parent id; None for non-merge commits.
pub fn merge_parents_label(commit: &CommitEntry) -> Option<String> {
  let [first, second, ..] = commit.parent_ids.as_slice() else {
    return None;
  };
  Some(format!("Merge: {}, {}", short_parent(first), short_parent(second)))
}

/// `Copy Commit ID ({shortId})`.
pub fn commit_id_menu_label(short_id: &str) -> String {
  format!("Copy Commit ID ({short_id})")
}

pub const RESET_MODES: [(&str, &str); 3] = [
  ("Reset (Soft)", "soft"),
  ("Reset (Mixed)", "mixed"),
  ("Reset (Hard)", "hard"),
];

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::FileStatus;

  fn commit_file(path: &str, status: FileStatus, old_path: Option<&str>) -> CommitFileEntry {
    CommitFileEntry {
      path: path.into(),
      status,
      old_path: old_path.map(str::to_string),
    }
  }

  fn commit(parent_ids: &[&str]) -> CommitEntry {
    CommitEntry {
      id: "0123456789abcdef0123456789abcdef01234567".into(),
      short_id: "0123456".into(),
      message: "subject".into(),
      author_name: "Ana Lima".into(),
      author_email: "ana@example.com".into(),
      author_date: "2026-09-01T00:00:00Z".into(),
      parent_ids: parent_ids.iter().map(|id| (*id).to_string()).collect(),
      avatar_url: String::new(),
    }
  }

  fn node(name: &str, path: &str, children: Vec<FileNode>, file: Option<CommitFileEntry>) -> FileNode {
    FileNode {
      name: name.into(),
      path: path.into(),
      children,
      file,
    }
  }

  #[test]
  fn initials_cases() {
    assert_eq!(initials("Ana Lima"), "AL");
    assert_eq!(initials("ana"), "A");
    assert_eq!(initials(""), "?");
    assert_eq!(initials("John Paul Jones"), "JP");
    assert_eq!(initials("   "), "?");
  }

  #[test]
  fn avatar_hue_is_stable_and_in_range() {
    let hue = avatar_hue("Ana Lima");
    assert_eq!(hue, avatar_hue("Ana Lima"));
    assert_eq!(hue, 300.0);
    assert_eq!(avatar_hue("ana"), 150.0);
    assert_eq!(avatar_hue(""), 150.0);
    assert!(AVATAR_HUES.contains(&hue));
    assert!(AVATAR_HUES.contains(&avatar_hue("Bob")));
  }

  #[test]
  fn changed_files_tree_nests_folders_first() {
    let readme = commit_file("README.md", FileStatus::Modified, None);
    let app = commit_file("App.rs", FileStatus::Added, None);
    let lib = commit_file("src/lib.rs", FileStatus::Modified, None);
    let deep = commit_file("src/nested/deep.rs", FileStatus::Modified, None);
    let renamed = commit_file("lib/z.rs", FileStatus::Renamed, Some("lib/old.rs"));
    let files = vec![lib.clone(), readme.clone(), deep.clone(), app.clone(), renamed.clone()];

    let tree = changed_files_tree(&files);
    assert_eq!(
      tree,
      vec![
        node(
          "lib",
          "lib",
          vec![node("z.rs", "lib/z.rs", vec![], Some(renamed))],
          None,
        ),
        node(
          "src",
          "src",
          vec![
            node(
              "nested",
              "src/nested",
              vec![node("deep.rs", "src/nested/deep.rs", vec![], Some(deep))],
              None,
            ),
            node("lib.rs", "src/lib.rs", vec![], Some(lib)),
          ],
          None,
        ),
        node("App.rs", "App.rs", vec![], Some(app)),
        node("README.md", "README.md", vec![], Some(readme)),
      ]
    );
  }

  #[test]
  fn merge_parents_label_for_merges_only() {
    assert_eq!(merge_parents_label(&commit(&[])), None);
    assert_eq!(merge_parents_label(&commit(&["aaaaaaaaaaaaaaaa"])), None);
    assert_eq!(
      merge_parents_label(&commit(&["abcdef1234567890", "xyzxyzx999999999", "1111111111111111"])),
      Some("Merge: abcdef1, xyzxyzx".into())
    );
  }

  #[test]
  fn commit_id_menu_label_format() {
    assert_eq!(commit_id_menu_label("abc1234"), "Copy Commit ID (abc1234)");
    assert_eq!(RESET_MODES[0], ("Reset (Soft)", "soft"));
    assert_eq!(RESET_MODES[1], ("Reset (Mixed)", "mixed"));
    assert_eq!(RESET_MODES[2], ("Reset (Hard)", "hard"));
  }
}
