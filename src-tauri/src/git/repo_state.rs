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
