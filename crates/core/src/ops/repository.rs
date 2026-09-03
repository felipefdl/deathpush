use std::collections::{HashSet, VecDeque};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::Core;
use crate::error::Result;
use crate::git::repository::GitRepository;
use crate::session::SessionId;
use crate::types::ProjectInfo;

#[derive(Default)]
pub struct RepoState {
  pub repo: Option<GitRepository>,
  pub cli_root: Option<PathBuf>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceScanEntry {
  pub directory: String,
  pub depth: u32,
}

fn scan_projects_in(path: &Path, depth: u32) -> Vec<ProjectInfo> {
  if !path.is_dir() {
    return Vec::new();
  }

  let mut projects = Vec::new();
  let mut queue: VecDeque<(PathBuf, u32)> = VecDeque::new();
  queue.push_back((path.to_path_buf(), 0));

  while let Some((dir, current_depth)) = queue.pop_front() {
    if current_depth > depth {
      continue;
    }

    if dir.join(".git").exists() {
      let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
      projects.push(ProjectInfo {
        path: dir.to_string_lossy().to_string(),
        name,
      });
      continue;
    }

    if current_depth < depth
      && let Ok(entries) = std::fs::read_dir(&dir)
    {
      for entry in entries.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
          let file_name = entry.file_name();
          let name = file_name.to_string_lossy();
          if !name.starts_with('.') {
            queue.push_back((entry_path, current_depth + 1));
          }
        }
      }
    }
  }

  projects
}

pub fn scan_workspace_projects(entries: &[WorkspaceScanEntry]) -> Result<Vec<ProjectInfo>> {
  let mut seen = HashSet::new();
  let mut projects = Vec::new();
  for entry in entries {
    for project in scan_projects_in(Path::new(&entry.directory), entry.depth) {
      if seen.insert(project.path.clone()) {
        projects.push(project);
      }
    }
  }
  projects.sort_by(|a, b| {
    a.name
      .to_lowercase()
      .cmp(&b.name.to_lowercase())
      .then_with(|| a.path.cmp(&b.path))
  });
  Ok(projects)
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NestedRepository {
  pub path: String,
  pub name: String,
  pub branch: Option<String>,
}

fn relative_repo_path(root: &Path, dir: &Path) -> String {
  dir
    .strip_prefix(root)
    .unwrap_or(dir)
    .components()
    .map(|component| component.as_os_str().to_string_lossy())
    .collect::<Vec<_>>()
    .join("/")
}

fn nested_repo_branch(path: &Path) -> Option<String> {
  let repo = git2::Repository::discover(path).ok()?;
  let head = repo.head().ok()?;
  if head.is_branch() {
    head.shorthand().ok().map(|s| s.to_string())
  } else {
    None
  }
}

fn discover_nested_in(root: &Path) -> Vec<NestedRepository> {
  let mut repos = Vec::new();
  let mut queue: VecDeque<PathBuf> = VecDeque::new();

  if let Ok(entries) = std::fs::read_dir(root) {
    for entry in entries.flatten() {
      let path = entry.path();
      if path.is_dir() && entry.file_name() != ".git" {
        queue.push_back(path);
      }
    }
  }

  while let Some(dir) = queue.pop_front() {
    if dir.join(".git").exists() {
      repos.push(NestedRepository {
        path: relative_repo_path(root, &dir),
        name: dir
          .file_name()
          .map(|name| name.to_string_lossy().to_string())
          .unwrap_or_default(),
        branch: nested_repo_branch(&dir),
      });
      continue;
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
      for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() && entry.file_name() != ".git" {
          queue.push_back(path);
        }
      }
    }
  }

  repos.sort_by(|a, b| {
    a.name
      .to_lowercase()
      .cmp(&b.name.to_lowercase())
      .then_with(|| a.path.cmp(&b.path))
  });
  repos
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
  pub path: String,
  pub name: String,
  pub branch: Option<String>,
  pub is_main: bool,
}

impl Core {
  pub fn discover_nested_repositories(&self, id: SessionId) -> Result<Vec<NestedRepository>> {
    let root = self.repo_root(id)?;
    Ok(discover_nested_in(&root))
  }

  pub fn detect_worktrees(&self, id: SessionId) -> Result<Vec<WorktreeInfo>> {
    let cli_root = self.repo_root(id)?;

    let repo = git2::Repository::open(&cli_root)?;

    let mut result = Vec::new();

    // Main worktree
    if let Some(workdir) = repo.workdir() {
      let name = workdir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
      let branch = repo.head().ok().and_then(|h| {
        if h.is_branch() {
          h.shorthand().ok().map(|s| s.to_string())
        } else {
          None
        }
      });
      result.push(WorktreeInfo {
        path: workdir.to_string_lossy().to_string(),
        name,
        branch,
        is_main: true,
      });
    }

    // Linked worktrees
    if let Ok(worktrees) = repo.worktrees() {
      let mut linked: Vec<WorktreeInfo> = Vec::new();
      for wt_name in worktrees.iter().filter_map(|n| n.ok().flatten()) {
        if let Ok(wt) = repo.find_worktree(wt_name) {
          let wt_path = wt.path().to_path_buf();
          let branch = git2::Repository::open(&wt_path).ok().and_then(|r| {
            r.head().ok().and_then(|h| {
              if h.is_branch() {
                h.shorthand().ok().map(|s| s.to_string())
              } else {
                None
              }
            })
          });
          let name = wt_path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| wt_name.to_string());
          linked.push(WorktreeInfo {
            path: wt_path.to_string_lossy().to_string(),
            name,
            branch,
            is_main: false,
          });
        }
      }
      linked.sort_by_key(|a| a.name.to_lowercase());
      result.extend(linked);
    }

    Ok(result)
  }
}

#[cfg(test)]
mod tests {
  use std::path::{Path, PathBuf};

  use tempfile::TempDir;

  use super::{WorkspaceScanEntry, discover_nested_in, scan_workspace_projects};

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

  fn init_with_commit(path: &Path) -> git2::Repository {
    std::fs::create_dir_all(path).unwrap();
    let repo = git2::Repository::init(path).unwrap();
    {
      let mut config = repo.config().unwrap();
      config.set_str("user.name", "Test").unwrap();
      config.set_str("user.email", "test@example.com").unwrap();
    }
    commit_file(&repo, "README.md", "hello\n");
    repo
  }

  fn checkout_branch(repo: &git2::Repository, name: &str) {
    let head = repo.head().unwrap().peel_to_commit().unwrap();
    repo.branch(name, &head, false).unwrap();
    repo.set_head(&format!("refs/heads/{name}")).unwrap();
    repo
      .checkout_head(Some(git2::build::CheckoutBuilder::new().force()))
      .unwrap();
  }

  #[test]
  fn discover_nested_returns_relative_path_name_and_branch() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    git2::Repository::init(root).unwrap();
    std::fs::create_dir_all(root.join("plain")).unwrap();

    let nested = root.join("nested");
    let repo = init_with_commit(&nested);
    checkout_branch(&repo, "topic");
    let _inner = init_with_commit(&nested.join("inner"));

    let found = discover_nested_in(root);
    assert_eq!(found.len(), 1, "inner repo under nested must not be listed: {found:?}");
    assert_eq!(found[0].path, "nested");
    assert_eq!(found[0].name, "nested");
    assert_eq!(found[0].branch.as_deref(), Some("topic"));
  }

  #[test]
  fn discover_nested_detached_head_has_null_branch() {
    let directory = TempDir::new().unwrap();
    let root = directory.path();
    git2::Repository::init(root).unwrap();
    let nested = root.join("detached");
    let repo = init_with_commit(&nested);
    let oid = repo.head().unwrap().peel_to_commit().unwrap().id();
    repo.set_head_detached(oid).unwrap();

    let found = discover_nested_in(root);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].path, "detached");
    assert!(found[0].branch.is_none(), "detached HEAD must not invent a branch name");
  }

  #[test]
  fn scan_workspace_skips_missing_dedupes_and_honors_depth() {
    let first = TempDir::new().unwrap();
    let second = TempDir::new().unwrap();
    let missing = PathBuf::from("/this/path/does/not/exist/deathpush-scan");

    let shallow = first.path().join("Zed");
    let deep_parent = first.path().join("mid");
    let deep = deep_parent.join("alpha");
    let overlap = second.path().join("alpha");
    let _ = init_with_commit(&shallow);
    let _ = init_with_commit(&deep);
    let _ = init_with_commit(&overlap);

    let projects = scan_workspace_projects(&[
      WorkspaceScanEntry {
        directory: first.path().to_string_lossy().to_string(),
        depth: 1,
      },
      WorkspaceScanEntry {
        directory: second.path().to_string_lossy().to_string(),
        depth: 1,
      },
      WorkspaceScanEntry {
        directory: missing.to_string_lossy().to_string(),
        depth: 1,
      },
    ])
    .unwrap();

    let names: Vec<&str> = projects.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
      names,
      vec!["alpha", "Zed"],
      "sort by lowercase name then path: {projects:?}"
    );
    assert_eq!(
      projects.len(),
      2,
      "depth 1 must skip mid/alpha and missing roots must not fail"
    );
    assert_eq!(projects[0].path, overlap.to_string_lossy().to_string());
    assert_eq!(projects[1].path, shallow.to_string_lossy().to_string());

    let with_deep = scan_workspace_projects(&[
      WorkspaceScanEntry {
        directory: first.path().to_string_lossy().to_string(),
        depth: 2,
      },
      WorkspaceScanEntry {
        directory: second.path().to_string_lossy().to_string(),
        depth: 1,
      },
    ])
    .unwrap();
    let deep_path = deep.to_string_lossy().into_owned();
    let deep_paths: Vec<&str> = with_deep.iter().map(|p| p.path.as_str()).collect();
    assert!(deep_paths.contains(&deep_path.as_str()));
    assert_eq!(
      with_deep.iter().filter(|p| p.name == "alpha").count(),
      2,
      "same name different paths stay distinct"
    );
  }

  #[test]
  fn scan_workspace_first_wins_on_overlapping_absolute_paths() {
    let directory = TempDir::new().unwrap();
    let project = directory.path().join("repo");
    let _ = init_with_commit(&project);

    let projects = scan_workspace_projects(&[
      WorkspaceScanEntry {
        directory: directory.path().to_string_lossy().to_string(),
        depth: 1,
      },
      WorkspaceScanEntry {
        directory: project.to_string_lossy().to_string(),
        depth: 0,
      },
    ])
    .unwrap();

    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].path, project.to_string_lossy().to_string());
  }
}
