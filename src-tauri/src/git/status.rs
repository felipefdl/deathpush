use std::collections::{BTreeMap, HashSet};
use std::path::Path;

use git2::{StatusOptions, StatusShow};

use crate::error::Result;
use crate::git::repo_state::detect_operation_state;
use crate::git::repository::GitRepository;
use crate::types::{
  FileEntry, FileStatus, RepositoryMetadata, RepositoryStatus, ResourceGroup, ResourceGroupKind, StatusEntry, StatusKey,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StatusScope {
  Exact(String),
  Subtree(String),
  Repository,
}

pub struct StatusScan {
  pub entries: Vec<StatusEntry>,
  pub metadata: Option<RepositoryMetadata>,
}

#[derive(Debug, Default)]
pub struct ScopeIndex {
  repository: bool,
  exact: HashSet<String>,
  subtrees: HashSet<String>,
}

impl ScopeIndex {
  pub fn new(scopes: &[StatusScope]) -> Self {
    let mut index = Self::default();
    for scope in scopes {
      match scope {
        StatusScope::Repository => index.repository = true,
        StatusScope::Exact(path) => {
          index.exact.insert(normalize_relative(path));
        }
        StatusScope::Subtree(path) => {
          let trimmed = normalize_relative(path);
          let trimmed = trimmed.trim_end_matches('/');
          if !trimmed.is_empty() {
            index.subtrees.insert(trimmed.to_string());
          }
        }
      }
    }
    index
  }

  pub fn contains(&self, path: &str) -> bool {
    if self.repository {
      return true;
    }
    self.exact.contains(path) || self.matches_subtree(path)
  }

  pub fn exact_paths(&self) -> impl Iterator<Item = &String> {
    self.exact.iter()
  }

  pub fn has_subtrees(&self) -> bool {
    !self.subtrees.is_empty()
  }

  pub fn matches_subtree(&self, path: &str) -> bool {
    let mut rest = path.trim_end_matches('/');
    while !rest.is_empty() {
      if self.subtrees.contains(rest) {
        return true;
      }
      match rest.rsplit_once('/') {
        Some((parent, _)) => rest = parent,
        None => break,
      }
    }
    false
  }
}

pub fn repository_status_from_entries(metadata: RepositoryMetadata, entries: &[StatusEntry]) -> RepositoryStatus {
  RepositoryStatus {
    root: metadata.root,
    head_branch: metadata.head_branch,
    head_commit: metadata.head_commit,
    ahead: metadata.ahead,
    behind: metadata.behind,
    groups: groups_from_entries(entries),
    operation_state: metadata.operation_state,
  }
}

#[allow(dead_code)]
pub fn get_repository_status(repo: &GitRepository) -> Result<RepositoryStatus> {
  let mut opts = status_options(true);
  let entries = scan_entries(repo, &mut opts)?;
  Ok(repository_status_from_entries(metadata_from(repo), &entries))
}

pub fn scan_baseline(root: &Path) -> Result<StatusScan> {
  let repo = GitRepository::open(root)?;
  let mut opts = status_options(true);
  Ok(StatusScan {
    entries: scan_entries(&repo, &mut opts)?,
    metadata: Some(metadata_from(&repo)),
  })
}

pub fn scan_scopes(root: &Path, scopes: &[StatusScope]) -> Result<StatusScan> {
  if scopes.iter().any(|scope| matches!(scope, StatusScope::Repository)) {
    return scan_baseline(root);
  }
  if scopes.is_empty() {
    return Ok(StatusScan {
      entries: Vec::new(),
      metadata: None,
    });
  }

  let repo = GitRepository::open(root)?;
  let mut merged: BTreeMap<StatusKey, StatusEntry> = BTreeMap::new();

  let exact: Vec<String> = scopes
    .iter()
    .filter_map(|scope| match scope {
      StatusScope::Exact(path) => Some(normalize_relative(path)),
      _ => None,
    })
    .collect();
  let subtree: Vec<String> = scopes
    .iter()
    .filter_map(|scope| match scope {
      StatusScope::Subtree(path) => Some(normalize_relative(path)),
      _ => None,
    })
    .collect();

  if !exact.is_empty() {
    let mut opts = status_options(false);
    opts.disable_pathspec_match(true);
    for path in &exact {
      opts.pathspec(path.as_str());
    }
    for entry in scan_entries(&repo, &mut opts)? {
      merged.insert(
        StatusKey {
          group: entry.group.clone(),
          path: entry.path.clone(),
        },
        entry,
      );
    }
  }

  if !subtree.is_empty() {
    let (special, normal): (Vec<&String>, Vec<&String>) = subtree.iter().partition(|path| has_pathspec_meta(path));
    if !normal.is_empty() {
      let mut opts = status_options(false);
      for path in &normal {
        opts.pathspec(path.as_str());
        let trimmed = path.trim_end_matches('/');
        if !path.ends_with('/') {
          opts.pathspec(format!("{trimmed}/"));
        }
      }
      for entry in scan_entries(&repo, &mut opts)? {
        merged.insert(
          StatusKey {
            group: entry.group.clone(),
            path: entry.path.clone(),
          },
          entry,
        );
      }
    }
    if !special.is_empty() {
      let scopes: Vec<StatusScope> = special
        .iter()
        .map(|path| StatusScope::Subtree((*path).clone()))
        .collect();
      let mut opts = status_options(false);
      for entry in scan_entries(&repo, &mut opts)? {
        if path_in_scopes(&entry.path, &scopes) {
          merged.insert(
            StatusKey {
              group: entry.group.clone(),
              path: entry.path.clone(),
            },
            entry,
          );
        }
      }
    }
  }

  Ok(StatusScan {
    entries: merged.into_values().collect(),
    metadata: None,
  })
}

pub fn path_in_scopes(path: &str, scopes: &[StatusScope]) -> bool {
  ScopeIndex::new(scopes).contains(path)
}

fn has_pathspec_meta(path: &str) -> bool {
  path.bytes().any(|byte| matches!(byte, b'*' | b'?' | b'[' | b'\\'))
}

fn normalize_relative(path: &str) -> String {
  path.replace('\\', "/")
}

fn status_options(rename: bool) -> StatusOptions {
  let mut opts = StatusOptions::new();
  opts
    .show(StatusShow::IndexAndWorkdir)
    .include_untracked(true)
    .recurse_untracked_dirs(true)
    .renames_head_to_index(rename)
    .renames_index_to_workdir(rename)
    .update_index(false);
  opts
}

fn scan_entries(repo: &GitRepository, opts: &mut StatusOptions) -> Result<Vec<StatusEntry>> {
  let statuses = repo.inner().statuses(Some(opts))?;
  Ok(collect_entries(&statuses))
}

fn metadata_from(repo: &GitRepository) -> RepositoryMetadata {
  let (ahead, behind) = repo.ahead_behind();
  RepositoryMetadata {
    root: repo.root().to_string_lossy().to_string(),
    head_branch: repo.head_branch(),
    head_commit: repo.head_commit_id(),
    ahead,
    behind,
    operation_state: detect_operation_state(repo.root()),
  }
}

fn collect_entries(statuses: &git2::Statuses<'_>) -> Vec<StatusEntry> {
  let mut entries = Vec::new();

  for entry in statuses.iter() {
    let path = entry.path().unwrap_or("").to_string();
    let s = entry.status();
    let head_to_index = entry.head_to_index();
    let index_to_workdir = entry.index_to_workdir();

    let rename_path_index = head_to_index.and_then(|d| d.new_file().path().map(|p| p.to_string_lossy().to_string()));
    let rename_path_workdir =
      index_to_workdir.and_then(|d| d.new_file().path().map(|p| p.to_string_lossy().to_string()));

    if s.is_conflicted() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Merge,
        path: path.clone(),
        status: classify_conflict(s),
        rename_path: None,
      });
      continue;
    }

    if s.is_index_new() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Index,
        path: path.clone(),
        status: FileStatus::IndexAdded,
        rename_path: None,
      });
    } else if s.is_index_modified() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Index,
        path: path.clone(),
        status: FileStatus::IndexModified,
        rename_path: None,
      });
    } else if s.is_index_deleted() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Index,
        path: path.clone(),
        status: FileStatus::IndexDeleted,
        rename_path: None,
      });
    } else if s.is_index_renamed() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Index,
        path: path.clone(),
        status: FileStatus::IndexRenamed,
        rename_path: rename_path_index,
      });
    } else if s.is_index_typechange() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::Index,
        path: path.clone(),
        status: FileStatus::TypeChanged,
        rename_path: None,
      });
    }

    if s.is_wt_modified() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: path.clone(),
        status: FileStatus::Modified,
        rename_path: None,
      });
    } else if s.is_wt_deleted() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: path.clone(),
        status: FileStatus::Deleted,
        rename_path: None,
      });
    } else if s.is_wt_renamed() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: path.clone(),
        status: FileStatus::Renamed,
        rename_path: rename_path_workdir,
      });
    } else if s.is_wt_typechange() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: path.clone(),
        status: FileStatus::TypeChanged,
        rename_path: None,
      });
    } else if s.is_wt_new() {
      entries.push(StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: path.clone(),
        status: FileStatus::Untracked,
        rename_path: None,
      });
    }
  }

  entries
}

fn groups_from_entries(entries: &[StatusEntry]) -> Vec<ResourceGroup> {
  let mut merge_files = Vec::new();
  let mut index_files = Vec::new();
  let mut working_tree_files = Vec::new();

  for entry in entries {
    let file = FileEntry {
      path: entry.path.clone(),
      status: entry.status.clone(),
      rename_path: entry.rename_path.clone(),
    };
    match entry.group {
      ResourceGroupKind::Merge => merge_files.push(file),
      ResourceGroupKind::Index => index_files.push(file),
      ResourceGroupKind::WorkingTree | ResourceGroupKind::Untracked => working_tree_files.push(file),
    }
  }

  let mut groups = Vec::new();
  if !merge_files.is_empty() {
    groups.push(ResourceGroup {
      kind: ResourceGroupKind::Merge,
      label: "Merge Changes".into(),
      files: merge_files,
    });
  }
  if !index_files.is_empty() {
    groups.push(ResourceGroup {
      kind: ResourceGroupKind::Index,
      label: "Staged Changes".into(),
      files: index_files,
    });
  }
  if !working_tree_files.is_empty() {
    groups.push(ResourceGroup {
      kind: ResourceGroupKind::WorkingTree,
      label: "Changes".into(),
      files: working_tree_files,
    });
  }
  groups
}

fn classify_conflict(_s: git2::Status) -> FileStatus {
  FileStatus::BothModified
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use tempfile::TempDir;

  use super::{ScopeIndex, StatusScope, get_repository_status, scan_baseline, scan_scopes};
  use crate::git::repository::GitRepository;
  use crate::types::{FileStatus, ResourceGroupKind};

  fn commit_file(repo: &git2::Repository, relative: &str, contents: &str) {
    let root = repo.workdir().unwrap();
    if let Some(parent) = Path::new(relative).parent() {
      std::fs::create_dir_all(root.join(parent)).unwrap();
    }
    std::fs::write(root.join(relative), contents).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("Test", "test@example.com").unwrap();
    let parents: Vec<git2::Commit> = match repo.head() {
      Ok(head) => vec![head.peel_to_commit().unwrap()],
      Err(_) => vec![],
    };
    let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
    repo
      .commit(Some("HEAD"), &sig, &sig, "test", &tree, &parent_refs)
      .unwrap();
  }

  fn init_repo() -> (TempDir, PathBuf) {
    let directory = TempDir::new().unwrap();
    let repo = git2::Repository::init(directory.path()).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_file(&repo, "README.md", "hello\n");
    let root = directory.path().to_path_buf();
    (directory, root)
  }

  #[test]
  fn scan_scopes_matches_pathspec_metacharacter_names_literally() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("file[1].txt"), "weird\n").unwrap();
    std::fs::write(root.join("other.txt"), "other\n").unwrap();

    let scan = scan_scopes(&root, &[StatusScope::Exact("file[1].txt".into())]).unwrap();
    assert!(
      scan.metadata.is_none(),
      "scoped scans must not compute ahead/behind metadata"
    );
    let paths: Vec<&str> = scan.entries.iter().map(|entry| entry.path.as_str()).collect();
    assert!(
      paths.contains(&"file[1].txt"),
      "expected literal pathspec match, got {paths:?}"
    );
    assert!(
      !paths.contains(&"other.txt"),
      "scoped scan should not include unrelated paths, got {paths:?}"
    );
    assert!(
      scan
        .entries
        .iter()
        .any(|entry| entry.group == ResourceGroupKind::WorkingTree && entry.status == FileStatus::Untracked)
    );
  }

  #[test]
  fn scan_scopes_matches_subtree_metacharacter_names_literally() {
    let (_dir, root) = init_repo();
    std::fs::create_dir_all(root.join("src[1]")).unwrap();
    std::fs::write(root.join("src[1]/file.rs"), "fn x() {}\n").unwrap();
    std::fs::create_dir_all(root.join("src1")).unwrap();
    std::fs::write(root.join("src1/other.rs"), "fn y() {}\n").unwrap();

    let scan = scan_scopes(&root, &[StatusScope::Subtree("src[1]".into())]).unwrap();
    let paths: Vec<&str> = scan.entries.iter().map(|entry| entry.path.as_str()).collect();
    assert!(
      paths.iter().any(|path| path.starts_with("src[1]/")),
      "expected literal subtree match, got {paths:?}"
    );
    assert!(
      !paths.iter().any(|path| path.starts_with("src1/")),
      "subtree glob should not match src1/, got {paths:?}"
    );
  }

  #[test]
  fn scan_baseline_includes_untracked_and_metadata() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("new.rs"), "fn main() {}\n").unwrap();

    let scan = scan_baseline(&root).unwrap();
    let metadata = scan.metadata.expect("baseline scan includes metadata");
    assert_eq!(
      std::fs::canonicalize(&metadata.root).unwrap(),
      std::fs::canonicalize(&root).unwrap()
    );
    assert!(metadata.head_branch.is_some());
    assert!(
      scan
        .entries
        .iter()
        .any(|entry| entry.path == "new.rs" && entry.status == FileStatus::Untracked)
    );
  }

  #[test]
  fn get_repository_status_wraps_baseline_snapshot() {
    let (_dir, root) = init_repo();
    std::fs::write(root.join("new.rs"), "fn main() {}\n").unwrap();
    let repo = GitRepository::open(&root).unwrap();
    let status = get_repository_status(&repo).unwrap();
    assert!(
      status
        .groups
        .iter()
        .flat_map(|group| group.files.iter())
        .any(|file| file.path == "new.rs" && file.status == FileStatus::Untracked)
    );
  }

  #[test]
  fn scope_index_matches_exact_by_key_and_subtree_by_prefix() {
    let index = ScopeIndex::new(&[
      StatusScope::Exact("a.rs".into()),
      StatusScope::Subtree("src".into()),
      StatusScope::Exact("b.rs".into()),
    ]);
    assert!(index.contains("a.rs"));
    assert!(index.contains("b.rs"));
    assert!(index.contains("src"));
    assert!(index.contains("src/lib.rs"));
    assert!(!index.contains("c.rs"));
    assert!(!index.contains("src2/lib.rs"));
    assert!(index.has_subtrees());
    assert_eq!(index.exact_paths().count(), 2);
  }

  #[test]
  fn scope_index_matches_subtree_by_ancestor_membership() {
    let mut scopes: Vec<StatusScope> = (0..256)
      .map(|index| StatusScope::Subtree(format!("unrelated-{index}")))
      .collect();
    scopes.push(StatusScope::Subtree("src/nested".into()));
    scopes.push(StatusScope::Subtree("src/nested/".into()));
    scopes.push(StatusScope::Exact("top.rs".into()));
    let index = ScopeIndex::new(&scopes);

    assert!(index.contains("src/nested"));
    assert!(index.contains("src/nested/lib.rs"));
    assert!(index.contains("src/nested/a/b/c.rs"));
    assert!(index.matches_subtree("src/nested/a/b/c.rs"));
    assert!(index.matches_subtree("src/nested"));
    assert!(!index.contains("src"));
    assert!(!index.matches_subtree("src"));
    assert!(!index.contains("src/other.rs"));
    assert!(!index.contains("src/nested-extra/x.rs"));
    assert!(index.contains("top.rs"));
    assert!(!index.matches_subtree("top.rs"));
    assert!(index.contains("unrelated-0"));
    assert!(index.contains("unrelated-0/file.rs"));
    assert!(index.has_subtrees());
  }

  #[test]
  fn scan_scopes_omits_metadata_for_empty_scopes() {
    let (_dir, root) = init_repo();
    let scan = scan_scopes(&root, &[]).unwrap();
    assert!(scan.metadata.is_none());
    assert!(scan.entries.is_empty());
  }
}
