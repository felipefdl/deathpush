use serde::{Deserialize, Serialize};

use crate::types::{
  BranchEntry, CommitDetail, CommitEntry, DiffHunk, FileBlame, LastCommitInfo, RepoOperationState, ResourceGroup,
  ResourceGroupKind, StashEntry, StatusPhase, TagEntry,
};

pub const DEFAULT_REMOTE: &str = "origin";
pub const COMMIT_LOG_PAGE: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSnapshot {
  pub session_generation: u64,
  pub session_revision: u64,
  pub status_generation: u64,
  pub status_revision: u64,
  pub repo: SessionRepo,
  pub groups: Vec<ResourceGroup>,
  pub selection: SessionSelection,
  pub scm: SessionScm,
  pub actions: SessionActions,
  pub last_commit: Option<LastCommitInfo>,
  pub branches: Vec<BranchEntry>,
  pub stashes: Vec<StashEntry>,
  pub tags: Vec<TagEntry>,
  pub commit_log: Vec<CommitEntry>,
  pub commit_detail: Option<CommitDetail>,
  pub file_history_path: Option<String>,
  pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionRepo {
  pub root: String,
  pub head_branch: Option<String>,
  pub head_commit: Option<String>,
  pub ahead: usize,
  pub behind: usize,
  pub operation_state: RepoOperationState,
  pub phase: StatusPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionSelection {
  pub file: Option<FileSelection>,
  pub commit: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct FileSelection {
  pub path: String,
  pub staged: bool,
  pub group_kind: ResourceGroupKind,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase")]
pub struct SessionScm {
  pub amend_mode: bool,
  pub commit_message: String,
  pub file_filter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SessionActions {
  pub can_commit: bool,
  pub commit_label: String,
  pub commit_destructive: bool,
  pub can_stage_all: bool,
  pub can_unstage_all: bool,
  pub can_discard_all: bool,
  pub discard_all_destructive: bool,
  pub sync: SyncAction,
  pub operation: OperationActions,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SyncKind {
  Fetch,
  Pull,
  Push,
  PullThenPush,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SyncAction {
  pub enabled: bool,
  pub kind: SyncKind,
  pub destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct OperationActions {
  #[serde(rename = "continue")]
  pub continue_op: bool,
  pub abort: bool,
  pub skip: bool,
  pub abort_destructive: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Intent {
  OpenRepository {
    path: String,
  },
  CloneRepository {
    url: String,
    directory: String,
  },

  RefreshStatus,
  ClearFile,
  SetAmend {
    enabled: bool,
  },
  SetCommitMessage {
    message: String,
  },
  SetFileFilter {
    filter: String,
  },
  Stage {
    paths: Vec<String>,
  },
  StageAll,
  Unstage {
    paths: Vec<String>,
  },
  UnstageAll,
  Discard {
    paths: Vec<String>,
    confirmed: bool,
  },
  Commit {
    confirmed: bool,
  },
  CommitAndPush {
    confirmed: bool,
  },
  CommitAndSync {
    confirmed: bool,
  },
  Sync,
  Push {
    force: bool,
    confirmed: bool,
  },
  Pull {
    rebase: bool,
  },
  Fetch {
    prune: bool,
  },
  UndoCommit {
    confirmed: bool,
  },
  OperationContinue,
  OperationAbort,
  OperationSkip,
  StageHunk {
    hunk_id: String,
  },
  UnstageHunk {
    hunk_id: String,
  },
  DiscardHunk {
    hunk_id: String,
    confirmed: bool,
  },
  StageLines {
    path: String,
    start: usize,
    end: usize,
    staged: bool,
  },
  OpenScmDiff {
    path: String,
    staged: bool,
    #[serde(default)]
    group_kind: Option<ResourceGroupKind>,
  },
  OpenCommitDiff {
    commit: String,
    path: String,
  },
  OpenBlame {
    path: String,
  },

  ResolveConflict {
    path: String,
    contents: String,
  },

  StashSave {
    include_untracked: bool,
    staged_only: bool,
    message: Option<String>,
  },
  StashApply {
    index: usize,
  },
  StashPop {
    index: usize,
  },
  StashDrop {
    index: usize,
    confirmed: bool,
  },
  CheckoutBranch {
    name: String,
  },
  CreateBranch {
    name: String,
    from: Option<String>,
  },
  DeleteBranch {
    name: String,
    force: bool,
    confirmed: bool,
  },
  RenameBranch {
    old_name: String,
    new_name: String,
  },
  MergeBranch {
    name: String,
  },
  RebaseBranch {
    name: String,
  },
  DeleteRemoteBranch {
    name: String,
  },
  CreateTag {
    name: String,
    message: Option<String>,
    commit: Option<String>,
  },
  DeleteTag {
    name: String,
    confirmed: bool,
  },
  PushTag {
    name: String,
  },
  DeleteRemoteTag {
    name: String,
  },
  CherryPick {
    commit: String,
  },
  Reset {
    commit: String,
    mode: String,
    confirmed: bool,
  },
  LoadMoreLog,
  OpenFileHistory {
    path: String,
  },
  ClearFileHistory,
  SelectCommit {
    id: String,
  },
  DeleteFile {
    path: String,
    confirmed: bool,
  },
  AddToGitignore {
    path: String,
  },
}

impl Intent {
  /// The same intent with `confirmed: true`, for resending after `IntentOutcome::NeedsConfirmation`.
  /// Variants without a confirmation flag come back unchanged.
  pub fn confirmed(self) -> Self {
    match self {
      Intent::Discard { paths, .. } => Intent::Discard { paths, confirmed: true },
      Intent::Commit { .. } => Intent::Commit { confirmed: true },
      Intent::CommitAndPush { .. } => Intent::CommitAndPush { confirmed: true },
      Intent::CommitAndSync { .. } => Intent::CommitAndSync { confirmed: true },
      Intent::Push { force, .. } => Intent::Push { force, confirmed: true },
      Intent::UndoCommit { .. } => Intent::UndoCommit { confirmed: true },
      Intent::DiscardHunk { hunk_id, .. } => Intent::DiscardHunk {
        hunk_id,
        confirmed: true,
      },
      Intent::StashDrop { index, .. } => Intent::StashDrop { index, confirmed: true },
      Intent::DeleteBranch { name, force, .. } => Intent::DeleteBranch {
        name,
        force,
        confirmed: true,
      },
      Intent::DeleteTag { name, .. } => Intent::DeleteTag { name, confirmed: true },
      Intent::Reset { commit, mode, .. } => Intent::Reset {
        commit,
        mode,
        confirmed: true,
      },
      Intent::DeleteFile { path, .. } => Intent::DeleteFile { path, confirmed: true },
      other => other,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum SessionPatch {
  Scm {
    scm: SessionScm,
    actions: SessionActions,
  },
  Actions {
    actions: SessionActions,
  },
  FileHistory {
    path: Option<String>,
    commit_log: Vec<CommitEntry>,
  },
  CommitLog {
    commit_log: Vec<CommitEntry>,
  },
  Commit {
    id: Option<String>,
    detail: Option<CommitDetail>,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusExtras {
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub last_commit: Option<LastCommitInfo>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub branches: Option<Vec<BranchEntry>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub tags: Option<Vec<TagEntry>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub commit_log: Option<Vec<CommitEntry>>,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub stashes: Option<Vec<StashEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatusEvent {
  pub session_generation: u64,
  pub session_revision: u64,
  pub status_generation: u64,
  pub status_revision: u64,
  pub repo: SessionRepo,
  pub groups: Vec<ResourceGroup>,
  pub actions: SessionActions,
  pub selection: SessionSelection,
  #[serde(default, skip_serializing_if = "Option::is_none")]
  pub extras: Option<SessionStatusExtras>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum IntentOutcome {
  Ack {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_generation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    session_revision: Option<u64>,
  },
  Patch {
    patch: SessionPatch,
    session_generation: u64,
    session_revision: u64,
  },
  Snapshot {
    snapshot: Box<SessionSnapshot>,
  },
  Diff {
    payload: DiffPayload,
    session_generation: u64,
    session_revision: u64,
  },
  Blame {
    payload: FileBlame,
    session_generation: u64,
    session_revision: u64,
  },
  NeedsConfirmation {
    action: String,
    message: String,
  },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffPayload {
  pub path: String,
  pub original: String,
  pub modified: String,
  pub language: Option<String>,
  pub file_type: String,
  pub hunks: Vec<DiffHunkPayload>,
  pub presence: DiffPresence,
  pub editable: bool,
  pub enable_line_selection: bool,
  pub staged: bool,
  pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunkPayload {
  pub id: String,
  pub header: String,
  pub old_start: usize,
  pub old_lines: usize,
  pub new_start: usize,
  pub new_lines: usize,
  pub lines: Vec<crate::types::DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DiffPresence {
  pub old_exists: bool,
  pub new_exists: bool,
}

impl From<&DiffHunk> for DiffHunkPayload {
  fn from(hunk: &DiffHunk) -> Self {
    Self {
      id: crate::git::hunk::hunk_id(hunk),
      header: hunk.header.clone(),
      old_start: hunk.old_start,
      old_lines: hunk.old_lines,
      new_start: hunk.new_start,
      new_lines: hunk.new_lines,
      lines: hunk.lines.clone(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn open_file_history_serializes_camel_case() {
    let json = serde_json::to_string(&Intent::OpenFileHistory {
      path: "src/a.rs".into(),
    })
    .unwrap();
    assert!(json.contains("\"openFileHistory\""));
    assert!(json.contains("\"path\":\"src/a.rs\""));
  }

  #[test]
  fn open_commit_diff_serializes_camel_case() {
    let json = serde_json::to_string(&Intent::OpenCommitDiff {
      commit: "abc123".into(),
      path: "src/a.rs".into(),
    })
    .unwrap();
    assert!(json.contains("\"openCommitDiff\""));
    assert!(json.contains("\"commit\":\"abc123\""));
  }

  #[test]
  fn clear_file_history_serializes_camel_case() {
    let json = serde_json::to_string(&Intent::ClearFileHistory).unwrap();
    assert!(json.contains("\"clearFileHistory\""));
  }

  #[test]
  fn clone_repository_serializes_camel_case() {
    let json = serde_json::to_string(&Intent::CloneRepository {
      url: "https://github.com/foo/bar.git".into(),
      directory: "/tmp".into(),
    })
    .unwrap();
    assert!(json.contains("\"cloneRepository\""));
    assert!(json.contains("\"directory\":\"/tmp\""));
  }

  #[test]
  fn open_blame_serializes_camel_case() {
    let json = serde_json::to_string(&Intent::OpenBlame {
      path: "src/a.rs".into(),
    })
    .unwrap();
    assert!(json.contains("\"openBlame\""));
  }

  #[test]
  fn select_file_is_not_a_valid_intent() {
    let json = r#"{"type":"selectFile","path":"src/a.rs","staged":false,"groupKind":"workingTree"}"#;
    assert!(serde_json::from_str::<Intent>(json).is_err());
  }

  #[test]
  fn open_scm_diff_deserializes_optional_group_kind() {
    let without = r#"{"type":"openScmDiff","path":"src/a.rs","staged":false}"#;
    assert_eq!(
      serde_json::from_str::<Intent>(without).unwrap(),
      Intent::OpenScmDiff {
        path: "src/a.rs".into(),
        staged: false,
        group_kind: None,
      }
    );
    let with = r#"{"type":"openScmDiff","path":"src/a.rs","staged":true,"groupKind":"index"}"#;
    assert_eq!(
      serde_json::from_str::<Intent>(with).unwrap(),
      Intent::OpenScmDiff {
        path: "src/a.rs".into(),
        staged: true,
        group_kind: Some(ResourceGroupKind::Index),
      }
    );
  }

  #[test]
  fn patch_outcome_serializes_session_revision() {
    let json = serde_json::to_string(&IntentOutcome::Patch {
      patch: SessionPatch::Actions {
        actions: SessionActions {
          can_commit: true,
          commit_label: "Commit".into(),
          commit_destructive: false,
          can_stage_all: false,
          can_unstage_all: false,
          can_discard_all: false,
          discard_all_destructive: true,
          sync: SyncAction {
            enabled: true,
            kind: SyncKind::Fetch,
            destructive: false,
          },
          operation: OperationActions {
            continue_op: false,
            abort: false,
            skip: false,
            abort_destructive: true,
          },
        },
      },
      session_generation: 3,
      session_revision: 7,
    })
    .unwrap();
    assert!(json.contains("\"sessionGeneration\":3"), "{json}");
    assert!(json.contains("\"sessionRevision\":7"), "{json}");
    assert!(json.contains("\"kind\":\"patch\""), "{json}");
  }
}

#[cfg(test)]
mod confirmed_tests {
  use super::Intent;

  #[test]
  fn confirmed_sets_the_flag_and_keeps_fields() {
    assert!(matches!(
      Intent::UndoCommit { confirmed: false }.confirmed(),
      Intent::UndoCommit { confirmed: true }
    ));
    assert!(matches!(
      Intent::Discard { paths: vec!["a".into()], confirmed: false }.confirmed(),
      Intent::Discard { paths, confirmed: true } if paths == vec!["a".to_string()]
    ));
    assert!(matches!(Intent::RefreshStatus.confirmed(), Intent::RefreshStatus));
    assert!(matches!(
      Intent::Commit { confirmed: false }.confirmed(),
      Intent::Commit { confirmed: true }
    ));
    assert!(matches!(
      Intent::CommitAndPush { confirmed: false }.confirmed(),
      Intent::CommitAndPush { confirmed: true }
    ));
    assert!(matches!(
      Intent::CommitAndSync { confirmed: false }.confirmed(),
      Intent::CommitAndSync { confirmed: true }
    ));
    assert!(matches!(
      Intent::Push {
        force: true,
        confirmed: false
      }
      .confirmed(),
      Intent::Push {
        force: true,
        confirmed: true
      }
    ));
    assert!(matches!(
      Intent::DiscardHunk { hunk_id: "h".into(), confirmed: false }.confirmed(),
      Intent::DiscardHunk { hunk_id, confirmed: true } if hunk_id == "h"
    ));
    assert!(matches!(
      Intent::StashDrop {
        index: 2,
        confirmed: false
      }
      .confirmed(),
      Intent::StashDrop {
        index: 2,
        confirmed: true
      }
    ));
    assert!(matches!(
      Intent::DeleteBranch { name: "x".into(), force: true, confirmed: false }.confirmed(),
      Intent::DeleteBranch { name, force: true, confirmed: true } if name == "x"
    ));
    assert!(matches!(
      Intent::DeleteTag { name: "v".into(), confirmed: false }.confirmed(),
      Intent::DeleteTag { name, confirmed: true } if name == "v"
    ));
    assert!(matches!(
      Intent::Reset { commit: "abc".into(), mode: "hard".into(), confirmed: false }.confirmed(),
      Intent::Reset { commit, mode, confirmed: true } if commit == "abc" && mode == "hard"
    ));
    assert!(matches!(
      Intent::DeleteFile { path: "a.rs".into(), confirmed: false }.confirmed(),
      Intent::DeleteFile { path, confirmed: true } if path == "a.rs"
    ));
  }
}
