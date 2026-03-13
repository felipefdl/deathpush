use std::collections::VecDeque;
use std::path::PathBuf;

use deathpush_core::error::Error;
use deathpush_core::git::repository::GitRepository;
use deathpush_core::git::status::get_repository_status;
use deathpush_core::git::watcher;
use deathpush_core::types::{ProjectInfo, RepositoryStatus};

use crate::session::{get_event_sink, manager};

#[derive(Debug, Clone, uniffi::Record)]
pub struct DiscoveredRepo {
  pub path: String,
  pub name: String,
}

#[derive(Debug, Clone, uniffi::Record)]
pub struct WorktreeInfo {
  pub path: String,
  pub name: String,
  pub branch: Option<String>,
  pub is_main: bool,
}

#[uniffi::export]
pub fn open_repository(session_id: String, path: String) -> Result<RepositoryStatus, Error> {
  let mgr = manager();
  let repo = GitRepository::open(&PathBuf::from(&path))?;
  let repo_root = repo.root().to_path_buf();

  // Store session first so get_repository_status can access the repo
  {
    let mut sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
    let state = sessions.entry(session_id.clone()).or_default();
    state.cli_root = Some(repo_root.clone());
    state.repo = Some(repo);
  }

  // Build full status including file change groups
  let status = {
    let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
    let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
    let repo = state.repo.as_ref().ok_or(Error::NoRepository)?;
    get_repository_status(repo)?
  };

  // Stop old watcher for this session, start new one
  {
    let mut watchers = mgr.watcher_state.lock().map_err(|e| Error::other(e.to_string()))?;
    watchers.remove(&session_id);
  }
  if let Some(sink) = get_event_sink() {
    if let Err(err) = watcher::start_watcher(&session_id, sink, &repo_root, &mgr.watcher_state) {
      tracing::warn!("failed to start watcher: {:?}", err);
    }
  }

  Ok(status)
}

#[uniffi::export]
pub fn scan_projects_directory(path: String, depth: u32) -> Result<Vec<ProjectInfo>, Error> {
  let root = PathBuf::from(&path);
  if !root.is_dir() {
    return Err(Error::other(format!("Not a directory: {}", path)));
  }

  let mut projects = Vec::new();
  let mut queue: VecDeque<(PathBuf, u32)> = VecDeque::new();
  queue.push_back((root, 0));

  while let Some((dir, current_depth)) = queue.pop_front() {
    if current_depth > depth {
      continue;
    }

    let git_dir = dir.join(".git");
    if git_dir.exists() {
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

    if current_depth < depth {
      if let Ok(entries) = std::fs::read_dir(&dir) {
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
  }

  projects.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
  Ok(projects)
}

#[uniffi::export]
pub fn discover_repositories(session_id: String) -> Result<Vec<DiscoveredRepo>, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let cli_root = state.cli_root.as_ref().ok_or(Error::NoRepository)?;

  let mut repos = Vec::new();
  let mut queue: VecDeque<PathBuf> = VecDeque::new();

  if let Ok(entries) = std::fs::read_dir(cli_root) {
    for entry in entries.flatten() {
      let p = entry.path();
      if p.is_dir() {
        let fname = entry.file_name();
        let name = fname.to_string_lossy();
        if name == ".git" {
          continue;
        }
        queue.push_back(p);
      }
    }
  }

  while let Some(dir) = queue.pop_front() {
    if dir.join(".git").exists() {
      let rel = dir.strip_prefix(cli_root).unwrap_or(&dir).to_string_lossy().to_string();
      let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
      repos.push(DiscoveredRepo { path: rel, name });
      continue;
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
      for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
          let fname = entry.file_name();
          let name = fname.to_string_lossy();
          if name == ".git" {
            continue;
          }
          queue.push_back(p);
        }
      }
    }
  }

  repos.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
  Ok(repos)
}

#[uniffi::export]
pub fn detect_worktrees(session_id: String) -> Result<Vec<WorktreeInfo>, Error> {
  let mgr = manager();
  let sessions = mgr.sessions.lock().map_err(|e| Error::other(e.to_string()))?;
  let state = sessions.get(&session_id).ok_or(Error::NoRepository)?;
  let cli_root = state.cli_root.as_ref().ok_or(Error::NoRepository)?;

  let repo = git2::Repository::open(cli_root)?;

  let mut result = Vec::new();

  // Main worktree
  if let Some(workdir) = repo.workdir() {
    let name = workdir
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();
    let branch = repo.head().ok().and_then(|h| {
      if h.is_branch() {
        h.shorthand().map(|s| s.to_string())
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
    for wt_name in worktrees.iter().flatten() {
      if let Ok(wt) = repo.find_worktree(wt_name) {
        let wt_path = wt.path().to_path_buf();
        let branch = git2::Repository::open(&wt_path).ok().and_then(|r| {
          r.head().ok().and_then(|h| {
            if h.is_branch() {
              h.shorthand().map(|s| s.to_string())
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
    linked.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    result.extend(linked);
  }

  Ok(result)
}

#[uniffi::export]
pub fn get_repo_branch(path: String) -> Result<Option<String>, Error> {
  let repo = git2::Repository::discover(&path)?;
  let branch = repo.head().ok().and_then(|h| {
    if h.is_branch() {
      h.shorthand().map(|s| s.to_string())
    } else {
      None
    }
  });
  Ok(branch)
}
