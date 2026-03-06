use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::Mutex;

use tauri::{State, WebviewWindow};

use crate::commands::update_window_title;
use crate::error::{Error, Result};
use crate::git::repository::GitRepository;
use crate::git::status::get_repository_status;
use crate::git::watcher::{self, WatcherState};
use crate::types::{ProjectInfo, RepositoryStatus};

pub struct CliArgs {
  pub path: Mutex<Option<String>>,
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
  let status = get_repository_status(&repo)?;

  // Stop old watcher for this window, start new one
  {
    let mut watchers = watcher_state.lock().map_err(|e| Error::Other(e.to_string()))?;
    watchers.remove(&label);
  }
  if let Err(err) = watcher::start_watcher(&window, &repo_root, &watcher_state) {
    tracing::warn!("failed to start watcher: {:?}", err);
  }

  update_window_title(&window, &status);

  let mut guard = state.lock().map_err(|e| Error::Other(e.to_string()))?;
  let win_state = guard.get_mut(&label);
  win_state.cli_root = Some(repo_root);
  win_state.repo = Some(repo);

  Ok(status)
}

#[tauri::command]
pub fn get_initial_path(state: State<'_, CliArgs>) -> Option<String> {
  state.path.lock().ok().and_then(|mut p| p.take())
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
