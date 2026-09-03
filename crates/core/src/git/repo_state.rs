use std::path::Path;

use crate::types::RepoOperationState;

pub fn detect_operation_state(repo_root: &Path) -> RepoOperationState {
  let git_dir = repo_root.join(".git");

  if git_dir.join("MERGE_HEAD").exists() {
    return RepoOperationState::Merging;
  }

  if git_dir.join("rebase-merge").exists() || git_dir.join("rebase-apply").exists() {
    return RepoOperationState::Rebasing;
  }

  if git_dir.join("CHERRY_PICK_HEAD").exists() {
    return RepoOperationState::CherryPicking;
  }

  if git_dir.join("REVERT_HEAD").exists() {
    return RepoOperationState::Reverting;
  }

  RepoOperationState::None
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_empty_git_dir_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join(".git")).unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::None);
  }

  #[test]
  fn test_merge_head_returns_merging() {
    let dir = tempfile::tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(git_dir.join("MERGE_HEAD"), "").unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::Merging);
  }

  #[test]
  fn test_rebase_merge_returns_rebasing() {
    let dir = tempfile::tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir_all(git_dir.join("rebase-merge")).unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::Rebasing);
  }

  #[test]
  fn test_rebase_apply_returns_rebasing() {
    let dir = tempfile::tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir_all(git_dir.join("rebase-apply")).unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::Rebasing);
  }

  #[test]
  fn test_cherry_pick_head_returns_cherry_picking() {
    let dir = tempfile::tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(git_dir.join("CHERRY_PICK_HEAD"), "").unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::CherryPicking);
  }

  #[test]
  fn test_revert_head_returns_reverting() {
    let dir = tempfile::tempdir().unwrap();
    let git_dir = dir.path().join(".git");
    std::fs::create_dir_all(&git_dir).unwrap();
    std::fs::write(git_dir.join("REVERT_HEAD"), "").unwrap();
    assert_eq!(detect_operation_state(dir.path()), RepoOperationState::Reverting);
  }
}
