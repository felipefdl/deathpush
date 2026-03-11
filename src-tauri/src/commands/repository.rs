use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;

use serde::Serialize;
use tauri::{Emitter, State, WebviewWindow};

use crate::commands::update_window_title;
use crate::error::{Error, Result};
use crate::git::repo_state::detect_operation_state;
use crate::git::repository::GitRepository;
use crate::git::watcher::{self, WatcherState};
use crate::types::{ProjectInfo, RepositoryStatus};

pub struct CliPaths {
  pub paths: Mutex<HashMap<String, String>>,
}

#[derive(Default)]
pub struct RepoState {
  pub repo: Option<GitRepository>,
  pub cli_root: Option<PathBuf>,
}

#[derive(Default)]
pub struct AppRepoState {
  pub windows: HashMap<String, RepoState>,
}

impl AppRepoState {
  pub fn get(&self, label: &str) -> Option<&RepoState> {
    self.windows.get(label)
  }

  pub fn get_mut(&mut self, label: &str) -> &mut RepoState {
    self.windows.entry(label.to_string()).or_default()
  }

  pub fn remove(&mut self, label: &str) {
    self.windows.remove(label);
  }
}

#[tauri::command]
pub fn open_repository(
  path: String,
  state: State<'_, Mutex<AppRepoState>>,
  watcher_state: State<'_, WatcherState>,
  window: WebviewWindow,
) -> Result<RepositoryStatus> {
  let label = window.label().to_string();
  let repo = GitRepository::open(&PathBuf::from(&path))?;
  let repo_root = repo.root().to_path_buf();

  // Build a fast status without scanning the working tree (groups are empty).
  // The frontend will follow up with get_status() to populate file lists.
  let (ahead, behind) = repo.ahead_behind();
  let status = RepositoryStatus {
    root: repo.root().to_string_lossy().to_string(),
    head_branch: repo.head_branch(),
    head_commit: repo.head_commit_id(),
    ahead,
    behind,
    groups: vec![],
    operation_state: detect_operation_state(repo.root()),
  };

  // Stop old watcher for this window, start new one
  {
    let mut watchers = watcher_state.lock().map_err(|e| Error::Other(e.to_string()))?;
    watchers.remove(&label);
  }
  if let Err(err) = watcher::start_watcher(&window, &repo_root, &watcher_state) {
    tracing::warn!("failed to start watcher: {:?}", err);
    let _ = window.emit(
      "watcher:error",
      format!("File watching unavailable: {}. Changes won't auto-refresh.", err),
    );
  }

  update_window_title(&window, &status);

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(status)
}

#[tauri::command]
pub fn get_initial_path(state: State<'_, CliPaths>, window: WebviewWindow) -> Option<String> {
  state.paths.lock().ok().and_then(|mut map| map.remove(window.label()))
}

#[tauri::command]
pub fn scan_projects_directory(path: String, depth: u32) -> Result<Vec<ProjectInfo>> {
  let root = PathBuf::from(&path);
  if !root.is_dir() {
    return Err(Error::Other(format!("Not a directory: {}", path)));
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiscoveredRepo {
  pub path: String,
  pub name: String,
}

#[tauri::command]
pub fn discover_repositories(
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<DiscoveredRepo>> {
  let label = window.label().to_string();
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
  let cli_root = win_state.cli_root.as_ref().ok_or(Error::NoRepository)?;

  let mut repos = Vec::new();
  let mut queue: VecDeque<PathBuf> = VecDeque::new();

  // Seed with immediate children of the repo root (skip .git itself)
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

  // BFS: find nested directories that contain a `.git` (sub-repos)
  while let Some(dir) = queue.pop_front() {
    if dir.join(".git").exists() {
      let rel = dir
        .strip_prefix(cli_root)
        .unwrap_or(&dir)
        .to_string_lossy()
        .to_string();
      let name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
      repos.push(DiscoveredRepo {
        path: rel,
        name,
      });
      // Don't recurse into sub-repos
      continue;
    }

    if let Ok(entries) = std::fs::read_dir(&dir) {
      for entry in entries.flatten() {
        let p = entry.path();
        if p.is_dir() {
          let fname = entry.file_name();
          let name = fname.to_string_lossy();
          // Skip .git dirs but allow other dot-prefixed dirs (they may be sub-repos)
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

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorktreeInfo {
  pub path: String,
  pub name: String,
  pub branch: Option<String>,
  pub is_main: bool,
}

#[tauri::command]
pub fn detect_worktrees(
  state: State<'_, Mutex<AppRepoState>>,
  window: WebviewWindow,
) -> Result<Vec<WorktreeInfo>> {
  let label = window.label().to_string();
  let guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get(&label).ok_or(Error::NoRepository)?;
  let cli_root = win_state.cli_root.as_ref().ok_or(Error::NoRepository)?;

  let repo = git2::Repository::open(cli_root)?;

  let mut result = Vec::new();

  // Main worktree
  if let Some(workdir) = repo.workdir() {
    let name = workdir
      .file_name()
      .map(|n| n.to_string_lossy().to_string())
      .unwrap_or_default();
    let branch = repo
      .head()
      .ok()
      .and_then(|h| if h.is_branch() { h.shorthand().map(|s| s.to_string()) } else { None });
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
        let branch = git2::Repository::open(&wt_path)
          .ok()
          .and_then(|r| {
            r.head()
              .ok()
              .and_then(|h| if h.is_branch() { h.shorthand().map(|s| s.to_string()) } else { None })
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

#[tauri::command]
pub fn get_repo_branch(path: String) -> Result<Option<String>> {
  let repo = git2::Repository::discover(&path)?;
  let branch = repo
    .head()
    .ok()
    .and_then(|h| if h.is_branch() { h.shorthand().map(|s| s.to_string()) } else { None });
  Ok(branch)
}
