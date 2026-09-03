use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
  pub content: String,
  pub line_type: String,
  pub old_line_number: Option<usize>,
  pub new_line_number: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
  pub header: String,
  pub old_start: usize,
  pub old_lines: usize,
  pub new_start: usize,
  pub new_lines: usize,
  pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FileStatus {
  Modified,
  Added,
  Deleted,
  Renamed,
  Copied,
  Untracked,
  Ignored,
  TypeChanged,
  IndexModified,
  IndexAdded,
  IndexDeleted,
  IndexRenamed,
  IndexCopied,
  IntentToAdd,
  IntentToRename,
  BothDeleted,
  AddedByUs,
  DeletedByThem,
  AddedByThem,
  DeletedByUs,
  BothAdded,
  BothModified,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub enum ResourceGroupKind {
  Index,
  WorkingTree,
  Untracked,
  Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
  pub path: String,
  pub status: FileStatus,
  pub rename_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroup {
  pub kind: ResourceGroupKind,
  pub label: String,
  pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum RepoOperationState {
  None,
  Merging,
  Rebasing,
  CherryPicking,
  Reverting,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryStatus {
  pub root: String,
  pub head_branch: Option<String>,
  pub head_commit: Option<String>,
  pub ahead: usize,
  pub behind: usize,
  pub groups: Vec<ResourceGroup>,
  pub operation_state: RepoOperationState,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum StatusPhase {
  Scanning,
  Settled,
  Storm,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
pub struct StatusKey {
  pub group: ResourceGroupKind,
  pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusEntry {
  pub group: ResourceGroupKind,
  pub path: String,
  pub status: FileStatus,
  pub rename_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryMetadata {
  pub root: String,
  pub head_branch: Option<String>,
  pub head_commit: Option<String>,
  pub ahead: usize,
  pub behind: usize,
  pub operation_state: RepoOperationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusPatch {
  pub generation: u64,
  pub base_revision: u64,
  pub revision: u64,
  pub upserts: Vec<StatusEntry>,
  pub removals: Vec<StatusKey>,
  pub metadata: Option<RepositoryMetadata>,
  pub phase: StatusPhase,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct StatusSnapshot {
  pub generation: u64,
  pub revision: u64,
  pub phase: StatusPhase,
  pub entries: Vec<StatusEntry>,
  pub metadata: RepositoryMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PathChangeKind {
  Content,
  Git,
  Structural,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PathChangeScope {
  Exact,
  Subtree,
  Repository,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PathsChanged {
  pub paths: Vec<String>,
  pub kind: PathChangeKind,
  pub scope: PathChangeScope,
  pub generation: u64,
  pub storm: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiffContent {
  pub path: String,
  pub original: String,
  pub modified: String,
  pub original_language: Option<String>,
  pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchEntry {
  pub name: String,
  pub is_head: bool,
  pub is_remote: bool,
  pub upstream: Option<String>,
  pub ahead: usize,
  pub behind: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitEntry {
  pub id: String,
  pub short_id: String,
  pub message: String,
  pub author_name: String,
  pub author_email: String,
  pub author_date: String,
  pub parent_ids: Vec<String>,
  pub avatar_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileEntry {
  pub path: String,
  pub status: FileStatus,
  pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
  pub commit: CommitEntry,
  pub files: Vec<CommitFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitDiffContent {
  pub path: String,
  pub original: String,
  pub modified: String,
  pub language: Option<String>,
  pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
  pub index: usize,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
  pub name: String,
  pub message: Option<String>,
  pub target_id: String,
  pub is_annotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlameLineGroup {
  pub commit_id: String,
  pub short_id: String,
  pub author_name: String,
  pub author_email: String,
  pub author_date: String,
  pub summary: String,
  pub start_line: usize,
  pub end_line: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileBlame {
  pub path: String,
  pub line_groups: Vec<BlameLineGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastCommitInfo {
  pub short_id: String,
  pub message: String,
  pub author_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
  pub path: String,
  pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerEntry {
  pub name: String,
  pub path: String,
  pub is_directory: bool,
  pub is_symlink: bool,
  pub ignored: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
  pub path: String,
  pub content: String,
  pub language: Option<String>,
  pub file_type: String,
  pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteFileResult {
  pub content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FuzzyFileResult {
  pub path: String,
  pub score: u32,
  pub match_positions: Vec<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchResult {
  pub path: String,
  pub line_number: usize,
  pub column: usize,
  pub line_content: String,
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn file_status_serializes_to_camel_case() {
    let json = serde_json::to_string(&FileStatus::IndexModified).unwrap();
    assert_eq!(json, "\"indexModified\"");
  }

  #[test]
  fn repo_operation_state_serializes_to_camel_case() {
    let json = serde_json::to_string(&RepoOperationState::CherryPicking).unwrap();
    assert_eq!(json, "\"cherryPicking\"");
  }

  #[test]
  fn resource_group_kind_serializes_to_camel_case() {
    let json = serde_json::to_string(&ResourceGroupKind::WorkingTree).unwrap();
    assert_eq!(json, "\"workingTree\"");
  }

  #[test]
  fn repository_status_fields_serialize_as_camel_case() {
    let status = RepositoryStatus {
      root: "/tmp".to_string(),
      head_branch: Some("main".to_string()),
      head_commit: None,
      ahead: 0,
      behind: 0,
      groups: vec![],
      operation_state: RepoOperationState::None,
    };
    let json = serde_json::to_string(&status).unwrap();
    assert!(json.contains("\"headBranch\""));
    assert!(json.contains("\"headCommit\""));
    assert!(json.contains("\"operationState\""));
  }

  #[test]
  fn diff_hunk_and_line_fields_serialize_as_camel_case() {
    let hunk = DiffHunk {
      header: "@@ -1,3 +1,3 @@".to_string(),
      old_start: 1,
      old_lines: 3,
      new_start: 1,
      new_lines: 3,
      lines: vec![DiffLine {
        content: "hello".to_string(),
        line_type: "add".to_string(),
        old_line_number: None,
        new_line_number: Some(1),
      }],
    };
    let json = serde_json::to_string(&hunk).unwrap();
    assert!(json.contains("\"oldStart\""));
    assert!(json.contains("\"newLines\""));
    assert!(json.contains("\"lineType\""));
    assert!(json.contains("\"oldLineNumber\""));
    assert!(json.contains("\"newLineNumber\""));
  }

  #[test]
  fn status_patch_serializes_camel_case_and_phase() {
    let patch = StatusPatch {
      generation: 1,
      base_revision: 0,
      revision: 1,
      upserts: vec![StatusEntry {
        group: ResourceGroupKind::WorkingTree,
        path: "a.rs".into(),
        status: FileStatus::Modified,
        rename_path: None,
      }],
      removals: vec![StatusKey {
        group: ResourceGroupKind::Index,
        path: "b.rs".into(),
      }],
      metadata: None,
      phase: StatusPhase::Scanning,
    };
    let json = serde_json::to_string(&patch).unwrap();
    assert!(json.contains("\"baseRevision\""));
    assert!(json.contains("\"upserts\""));
    assert!(json.contains("\"removals\""));
    assert!(json.contains("\"scanning\""));
    assert!(json.contains("\"workingTree\""));
  }

  #[test]
  fn paths_changed_serializes_scope_and_storm() {
    let event = PathsChanged {
      paths: vec!["src/lib.rs".into()],
      kind: PathChangeKind::Content,
      scope: PathChangeScope::Exact,
      generation: 3,
      storm: true,
    };
    let json = serde_json::to_string(&event).unwrap();
    assert!(json.contains("\"exact\""));
    assert!(json.contains("\"content\""));
    assert!(json.contains("\"storm\":true"));
    assert!(json.contains("\"generation\":3"));
  }

  #[test]
  fn commit_file_entry_status_serializes_as_file_status() {
    let file = CommitFileEntry {
      path: "src/a.rs".into(),
      status: FileStatus::TypeChanged,
      old_path: Some("src/b.rs".into()),
    };
    let json = serde_json::to_string(&file).unwrap();
    assert!(json.contains("\"typeChanged\""));
    assert!(json.contains("\"oldPath\""));
    assert!(!json.contains("\"typechange\""));
  }
}
