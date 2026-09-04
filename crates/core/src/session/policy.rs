use crate::types::{FileStatus, RepoOperationState, ResourceGroup, ResourceGroupKind};
use std::path::{Path, PathBuf};

use super::types::{OperationActions, SessionActions, SyncAction, SyncKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscardPlan {
  pub tracked: Vec<String>,
  pub untracked: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationRoute {
  MergeContinue,
  MergeAbort,
  RebaseContinue,
  RebaseAbort,
  RebaseSkip,
  CherryPickContinue,
  CherryPickAbort,
  RevertContinue,
  RevertAbort,
}

pub fn has_staged(groups: &[ResourceGroup]) -> bool {
  groups
    .iter()
    .any(|group| group.kind == ResourceGroupKind::Index && !group.files.is_empty())
}

pub fn has_other_changes(groups: &[ResourceGroup]) -> bool {
  groups
    .iter()
    .any(|group| group.kind != ResourceGroupKind::Index && !group.files.is_empty())
}

pub fn commit_label(amend: bool, has_staged: bool, has_other_changes: bool) -> &'static str {
  match (amend, has_staged, has_other_changes) {
    (true, false, true) => "Amend All",
    (true, _, _) => "Amend",
    (false, false, true) => "Commit All",
    _ => "Commit",
  }
}

pub fn can_commit(message: &str, has_staged: bool, has_other_changes: bool) -> bool {
  !message.trim().is_empty() && (has_staged || has_other_changes)
}

pub fn should_stage_all_before_commit(has_staged: bool, has_other_changes: bool) -> bool {
  !has_staged && has_other_changes
}

pub fn sync_kind(ahead: usize, behind: usize) -> SyncKind {
  match (ahead > 0, behind > 0) {
    (true, true) => SyncKind::PullThenPush,
    (false, true) => SyncKind::Pull,
    (true, false) => SyncKind::Push,
    (false, false) => SyncKind::Fetch,
  }
}

pub fn sync_kind_after_commit(ahead: usize, behind: usize, amend: bool) -> SyncKind {
  let next_ahead = if amend { ahead } else { ahead.saturating_add(1) };
  sync_kind(next_ahead, behind)
}

pub fn sync_enabled(kind: SyncKind, has_branch: bool) -> bool {
  match kind {
    SyncKind::Fetch => true,
    SyncKind::Pull | SyncKind::Push | SyncKind::PullThenPush => has_branch,
  }
}

pub fn expand_resource_paths(files: &[(String, FileStatus)], selected: &[String]) -> Vec<String> {
  let mut resolved = Vec::new();
  for selected_path in selected {
    if selected_path.ends_with('/') {
      for (path, _) in files {
        if path.starts_with(selected_path) && !resolved.iter().any(|existing| existing == path) {
          resolved.push(path.clone());
        }
      }
    } else if files.iter().any(|(path, _)| path == selected_path)
      && !resolved.iter().any(|existing| existing == selected_path)
    {
      resolved.push(selected_path.clone());
    }
  }
  resolved
}

pub fn classify_discard(files: &[(String, FileStatus)], paths: &[String]) -> DiscardPlan {
  let expanded = expand_resource_paths(files, paths);
  let mut tracked = Vec::new();
  let mut untracked = Vec::new();
  for path in expanded {
    let untracked_file = files
      .iter()
      .find(|(candidate, _)| candidate == &path)
      .is_some_and(|(_, status)| *status == FileStatus::Untracked);
    if untracked_file {
      untracked.push(path);
    } else {
      tracked.push(path);
    }
  }
  DiscardPlan { tracked, untracked }
}

pub fn files_from_groups(groups: &[ResourceGroup]) -> Vec<(String, FileStatus)> {
  groups
    .iter()
    .flat_map(|group| group.files.iter().map(|file| (file.path.clone(), file.status.clone())))
    .collect()
}

pub fn unstaged_files(groups: &[ResourceGroup]) -> Vec<(String, FileStatus)> {
  groups
    .iter()
    .filter(|group| group.kind != ResourceGroupKind::Index)
    .flat_map(|group| group.files.iter().map(|file| (file.path.clone(), file.status.clone())))
    .collect()
}

pub fn discard_confirmation_message(plan: &DiscardPlan) -> (String, String) {
  let tracked = plan.tracked.len();
  let untracked = plan.untracked.len();
  if tracked > 0 && untracked > 0 {
    (
      format!(
        "Are you sure you want to discard {tracked} change(s) and DELETE {untracked} untracked file(s)?\n\nTracked changes are irreversible. Untracked files can be restored from the Trash."
      ),
      "discard".into(),
    )
  } else if untracked > 0 {
    let message = if untracked == 1 {
      let name = plan.untracked[0].rsplit('/').next().unwrap_or(&plan.untracked[0]);
      format!(
        "Are you sure you want to DELETE the following untracked file: '{name}'?\n\nYou can restore this file from the Trash."
      )
    } else {
      format!("Are you sure you want to DELETE {untracked} untracked file(s)?\n\nYou can restore them from the Trash.")
    };
    (message, "discard".into())
  } else if tracked == 1 {
    let name = plan.tracked[0].rsplit('/').next().unwrap_or(&plan.tracked[0]);
    (
      format!("Are you sure you want to discard changes in \"{name}\"?\n\nThis action is irreversible."),
      "discard".into(),
    )
  } else {
    (
      format!("Are you sure you want to discard all {tracked} change(s)?\n\nThis action is irreversible."),
      "discard".into(),
    )
  }
}

pub fn operation_continue(state: RepoOperationState) -> Option<OperationRoute> {
  match state {
    RepoOperationState::Merging => Some(OperationRoute::MergeContinue),
    RepoOperationState::Rebasing => Some(OperationRoute::RebaseContinue),
    RepoOperationState::CherryPicking => Some(OperationRoute::CherryPickContinue),
    RepoOperationState::Reverting => Some(OperationRoute::RevertContinue),
    RepoOperationState::None => None,
  }
}

pub fn operation_abort(state: RepoOperationState) -> Option<OperationRoute> {
  match state {
    RepoOperationState::Merging => Some(OperationRoute::MergeAbort),
    RepoOperationState::Rebasing => Some(OperationRoute::RebaseAbort),
    RepoOperationState::CherryPicking => Some(OperationRoute::CherryPickAbort),
    RepoOperationState::Reverting => Some(OperationRoute::RevertAbort),
    RepoOperationState::None => None,
  }
}

pub fn operation_skip(state: RepoOperationState) -> Option<OperationRoute> {
  match state {
    RepoOperationState::Rebasing => Some(OperationRoute::RebaseSkip),
    _ => None,
  }
}

pub fn scm_patch_presence(group_kind: ResourceGroupKind, status: Option<&FileStatus>) -> (bool, bool) {
  if group_kind == ResourceGroupKind::Untracked {
    return (false, true);
  }
  match status {
    Some(FileStatus::Deleted | FileStatus::IndexDeleted) => (true, false),
    Some(FileStatus::Added | FileStatus::IndexAdded | FileStatus::IntentToAdd) => (false, true),
    _ => (true, true),
  }
}

pub fn is_scm_diff_editable(group_kind: ResourceGroupKind, has_working_tree_side: bool) -> bool {
  group_kind != ResourceGroupKind::Index && group_kind != ResourceGroupKind::Merge && has_working_tree_side
}

pub fn enable_scm_line_selection(group_kind: ResourceGroupKind) -> bool {
  matches!(group_kind, ResourceGroupKind::WorkingTree | ResourceGroupKind::Index)
}

pub fn derive_actions(
  groups: &[ResourceGroup],
  message: &str,
  amend: bool,
  ahead: usize,
  behind: usize,
  has_branch: bool,
  operation_state: RepoOperationState,
) -> SessionActions {
  let staged = has_staged(groups);
  let other = has_other_changes(groups);
  let kind = sync_kind(ahead, behind);
  SessionActions {
    can_commit: can_commit(message, staged, other),
    commit_label: commit_label(amend, staged, other).to_string(),
    commit_destructive: amend,
    can_stage_all: other,
    can_unstage_all: staged,
    can_discard_all: other,
    discard_all_destructive: true,
    sync: SyncAction {
      enabled: sync_enabled(kind, has_branch),
      kind,
      destructive: false,
    },
    operation: OperationActions {
      continue_op: operation_continue(operation_state).is_some(),
      abort: operation_abort(operation_state).is_some(),
      skip: operation_skip(operation_state).is_some(),
      abort_destructive: true,
    },
  }
}

pub fn confirmation_required(confirmed: bool) -> bool {
  !confirmed
}

pub fn push_needs_confirmation(force: bool, confirmed: bool) -> bool {
  force && confirmation_required(confirmed)
}

pub fn reset_needs_confirmation(_mode: &str, confirmed: bool) -> bool {
  confirmation_required(confirmed)
}

pub fn repo_name_from_url(url: &str) -> String {
  let trimmed = url.trim().trim_end_matches('/');
  let last = trimmed.rsplit(['/', ':']).next().unwrap_or("");
  let name = last.strip_suffix(".git").unwrap_or(last);
  if name.is_empty() { "repo".into() } else { name.into() }
}

pub fn clone_target_path(url: &str, directory: &str) -> PathBuf {
  Path::new(directory).join(repo_name_from_url(url))
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::types::{FileEntry, ResourceGroup};

  fn group(kind: ResourceGroupKind, files: &[(&str, FileStatus)]) -> ResourceGroup {
    ResourceGroup {
      kind,
      label: "x".into(),
      files: files
        .iter()
        .map(|(path, status)| FileEntry {
          path: (*path).into(),
          status: status.clone(),
          rename_path: None,
        })
        .collect(),
    }
  }

  #[test]
  fn commit_label_commit_all_when_nothing_staged() {
    assert_eq!(commit_label(false, false, true), "Commit All");
    assert_eq!(commit_label(false, true, true), "Commit");
    assert_eq!(commit_label(false, false, false), "Commit");
    assert_eq!(commit_label(true, false, true), "Amend All");
    assert_eq!(commit_label(true, true, false), "Amend");
  }

  #[test]
  fn can_commit_requires_message_and_changes() {
    assert!(!can_commit("  ", true, false));
    assert!(!can_commit("msg", false, false));
    assert!(can_commit("msg", true, false));
    assert!(can_commit("msg", false, true));
  }

  #[test]
  fn commit_all_policy_stages_when_index_empty() {
    assert!(should_stage_all_before_commit(false, true));
    assert!(!should_stage_all_before_commit(true, true));
    assert!(!should_stage_all_before_commit(false, false));
  }

  #[test]
  fn sync_kind_matches_toolbar_policy() {
    assert_eq!(sync_kind(0, 0), SyncKind::Fetch);
    assert_eq!(sync_kind(0, 2), SyncKind::Pull);
    assert_eq!(sync_kind(3, 0), SyncKind::Push);
    assert_eq!(sync_kind(1, 1), SyncKind::PullThenPush);
  }

  #[test]
  fn commit_and_sync_counts_the_new_commit() {
    assert_eq!(sync_kind_after_commit(0, 0, false), SyncKind::Push);
    assert_eq!(sync_kind_after_commit(0, 2, false), SyncKind::PullThenPush);
    assert_eq!(sync_kind_after_commit(0, 0, true), SyncKind::Fetch);
  }

  #[test]
  fn sync_push_requires_a_branch() {
    assert!(sync_enabled(SyncKind::Fetch, false));
    assert!(!sync_enabled(SyncKind::Push, false));
    assert!(sync_enabled(SyncKind::Push, true));
  }

  #[test]
  fn discard_classifies_untracked_separately() {
    let files = vec![
      ("a.rs".into(), FileStatus::Modified),
      ("tmp.log".into(), FileStatus::Untracked),
    ];
    let plan = classify_discard(&files, &["a.rs".into(), "tmp.log".into()]);
    assert_eq!(plan.tracked, vec!["a.rs"]);
    assert_eq!(plan.untracked, vec!["tmp.log"]);
  }

  #[test]
  fn discard_expands_directory_prefix() {
    let files = vec![
      ("src/a.rs".into(), FileStatus::Modified),
      ("src/b.rs".into(), FileStatus::Untracked),
      ("readme.md".into(), FileStatus::Modified),
    ];
    let plan = classify_discard(&files, &["src/".into()]);
    assert_eq!(plan.tracked, vec!["src/a.rs"]);
    assert_eq!(plan.untracked, vec!["src/b.rs"]);
  }

  #[test]
  fn operation_routes_cherry_pick_and_revert_away_from_merge() {
    assert_eq!(
      operation_continue(RepoOperationState::CherryPicking),
      Some(OperationRoute::CherryPickContinue)
    );
    assert_eq!(
      operation_abort(RepoOperationState::CherryPicking),
      Some(OperationRoute::CherryPickAbort)
    );
    assert_eq!(
      operation_continue(RepoOperationState::Reverting),
      Some(OperationRoute::RevertContinue)
    );
    assert_eq!(
      operation_abort(RepoOperationState::Reverting),
      Some(OperationRoute::RevertAbort)
    );
    assert_eq!(
      operation_continue(RepoOperationState::Merging),
      Some(OperationRoute::MergeContinue)
    );
    assert_eq!(
      operation_skip(RepoOperationState::Rebasing),
      Some(OperationRoute::RebaseSkip)
    );
    assert_eq!(operation_skip(RepoOperationState::Merging), None);
    assert_eq!(operation_continue(RepoOperationState::None), None);
  }

  #[test]
  fn confirm_gate_rejects_unconfirmed_destructive_ops() {
    assert!(confirmation_required(false));
    assert!(!confirmation_required(true));
  }

  #[test]
  fn derive_actions_sets_commit_all_and_sync_fetch() {
    let groups = vec![group(ResourceGroupKind::WorkingTree, &[("a.rs", FileStatus::Modified)])];
    let actions = derive_actions(&groups, "msg", false, 0, 0, true, RepoOperationState::None);
    assert_eq!(actions.commit_label, "Commit All");
    assert!(actions.can_commit);
    assert!(actions.can_stage_all);
    assert!(!actions.can_unstage_all);
    assert_eq!(actions.sync.kind, SyncKind::Fetch);
    assert!(!actions.operation.continue_op);
  }

  #[test]
  fn derive_actions_rebase_enables_skip() {
    let actions = derive_actions(&[], "", false, 2, 1, true, RepoOperationState::Rebasing);
    assert_eq!(actions.sync.kind, SyncKind::PullThenPush);
    assert!(actions.operation.continue_op);
    assert!(actions.operation.abort);
    assert!(actions.operation.skip);
    assert!(actions.operation.abort_destructive);
  }

  #[test]
  fn scm_presence_and_editability() {
    assert_eq!(scm_patch_presence(ResourceGroupKind::Untracked, None), (false, true));
    assert_eq!(
      scm_patch_presence(ResourceGroupKind::WorkingTree, Some(&FileStatus::Deleted)),
      (true, false)
    );
    assert_eq!(
      scm_patch_presence(ResourceGroupKind::Index, Some(&FileStatus::IndexAdded)),
      (false, true)
    );
    assert!(is_scm_diff_editable(ResourceGroupKind::WorkingTree, true));
    assert!(!is_scm_diff_editable(ResourceGroupKind::Index, true));
    assert!(!is_scm_diff_editable(ResourceGroupKind::Merge, true));
    assert!(enable_scm_line_selection(ResourceGroupKind::Index));
    assert!(!enable_scm_line_selection(ResourceGroupKind::Merge));
  }

  #[test]
  fn discard_confirmation_copy() {
    let (message, action) = discard_confirmation_message(&DiscardPlan {
      tracked: vec!["a.rs".into()],
      untracked: vec!["tmp".into()],
    });
    assert_eq!(action, "discard");
    assert!(message.contains("DELETE"));
    assert!(message.contains("Trash"));
  }

  #[test]
  fn force_push_requires_confirmation() {
    assert!(push_needs_confirmation(true, false));
    assert!(!push_needs_confirmation(true, true));
    assert!(!push_needs_confirmation(false, false));
  }

  #[test]
  fn undo_commit_requires_confirmation() {
    assert!(confirmation_required(false));
    assert!(!confirmation_required(true));
  }

  #[test]
  fn reset_requires_confirmation() {
    assert!(reset_needs_confirmation("hard", false));
    assert!(!reset_needs_confirmation("hard", true));
    assert!(reset_needs_confirmation("soft", false));
    assert!(!reset_needs_confirmation("soft", true));
    assert!(reset_needs_confirmation("mixed", false));
    assert!(!reset_needs_confirmation("mixed", true));
  }

  #[test]
  fn intent_serializes_tagged_camel_case() {
    let json = serde_json::to_string(&crate::session::types::Intent::Pull { rebase: true }).unwrap();
    assert!(json.contains("\"type\":\"pull\""));
    assert!(json.contains("\"rebase\":true"));
    let refresh = serde_json::to_string(&crate::session::types::Intent::RefreshStatus).unwrap();
    assert_eq!(refresh, "{\"type\":\"refreshStatus\"}");
    let force = serde_json::to_string(&crate::session::types::Intent::Push {
      force: true,
      confirmed: false,
    })
    .unwrap();
    assert!(force.contains("\"type\":\"push\""));
    assert!(force.contains("\"force\":true"));
    let follow = serde_json::to_string(&crate::session::types::Intent::CommitAndSync { confirmed: false }).unwrap();
    assert!(follow.contains("\"type\":\"commitAndSync\""));
  }

  #[test]
  fn repo_name_from_url_strips_git_suffix() {
    assert_eq!(repo_name_from_url("https://github.com/foo/bar.git"), "bar");
    assert_eq!(repo_name_from_url("git@github.com:foo/bar.git"), "bar");
    assert_eq!(repo_name_from_url("https://github.com/foo/bar.git/"), "bar");
    assert_eq!(repo_name_from_url("   "), "repo");
  }

  #[test]
  fn clone_target_path_joins_directory() {
    assert_eq!(
      clone_target_path("https://github.com/foo/bar.git", "/tmp"),
      PathBuf::from("/tmp").join("bar")
    );
  }
}
