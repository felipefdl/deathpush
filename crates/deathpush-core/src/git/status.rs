use git2::{StatusOptions, StatusShow};

use crate::error::Result;
use crate::git::repo_state::detect_operation_state;
use crate::git::repository::GitRepository;
use crate::types::{FileEntry, FileStatus, RepositoryStatus, ResourceGroup, ResourceGroupKind};

pub fn get_repository_status(repo: &GitRepository) -> Result<RepositoryStatus> {
  let mut opts = StatusOptions::new();
  opts
    .show(StatusShow::IndexAndWorkdir)
    .include_untracked(true)
    .renames_head_to_index(true)
    .renames_index_to_workdir(true)
    .recurse_untracked_dirs(true);

  let statuses = repo.inner().statuses(Some(&mut opts))?;

  let mut index_files: Vec<FileEntry> = Vec::new();
  let mut working_tree_files: Vec<FileEntry> = Vec::new();
  let mut untracked_files: Vec<FileEntry> = Vec::new();
  let mut merge_files: Vec<FileEntry> = Vec::new();

  for entry in statuses.iter() {
    let path = entry.path().unwrap_or("").to_string();
    let s = entry.status();
    let head_to_index = entry.head_to_index();
    let index_to_workdir = entry.index_to_workdir();

    let rename_path_index = head_to_index.and_then(|d| d.new_file().path().map(|p| p.to_string_lossy().to_string()));
    let rename_path_workdir =
      index_to_workdir.and_then(|d| d.new_file().path().map(|p| p.to_string_lossy().to_string()));

    // Merge conflicts
    if s.is_conflicted() {
      let status = classify_conflict(s);
      merge_files.push(FileEntry {
        path: path.clone(),
        status,
        rename_path: None,
      });
      continue;
    }

    // Index (staged) changes
    if s.is_index_new() {
      index_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::IndexAdded,
        rename_path: None,
      });
    } else if s.is_index_modified() {
      index_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::IndexModified,
        rename_path: None,
      });
    } else if s.is_index_deleted() {
      index_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::IndexDeleted,
        rename_path: None,
      });
    } else if s.is_index_renamed() {
      index_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::IndexRenamed,
        rename_path: rename_path_index,
      });
    } else if s.is_index_typechange() {
      index_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::TypeChanged,
        rename_path: None,
      });
    }

    // Working tree (unstaged) changes
    if s.is_wt_modified() {
      working_tree_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::Modified,
        rename_path: None,
      });
    } else if s.is_wt_deleted() {
      working_tree_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::Deleted,
        rename_path: None,
      });
    } else if s.is_wt_renamed() {
      working_tree_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::Renamed,
        rename_path: rename_path_workdir,
      });
    } else if s.is_wt_typechange() {
      working_tree_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::TypeChanged,
        rename_path: None,
      });
    } else if s.is_wt_new() {
      untracked_files.push(FileEntry {
        path: path.clone(),
        status: FileStatus::Untracked,
        rename_path: None,
      });
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
  if !working_tree_files.is_empty() || !untracked_files.is_empty() {
    let mut changes = working_tree_files;
    changes.extend(untracked_files);
    groups.push(ResourceGroup {
      kind: ResourceGroupKind::WorkingTree,
      label: "Changes".into(),
      files: changes,
    });
  }

  let (ahead, behind) = repo.ahead_behind();
  let operation_state = detect_operation_state(repo.root());

  Ok(RepositoryStatus {
    root: repo.root().to_string_lossy().to_string(),
    head_branch: repo.head_branch(),
    head_commit: repo.head_commit_id(),
    ahead: ahead as u32,
    behind: behind as u32,
    groups,
    operation_state,
  })
}

fn classify_conflict(_s: git2::Status) -> FileStatus {
  // git2 doesn't give us granular conflict info like git status --porcelain,
  // so we default to BothModified for all conflicts
  FileStatus::BothModified
}
