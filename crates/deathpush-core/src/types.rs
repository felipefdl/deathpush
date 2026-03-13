use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct DiffLine {
  pub content: String,
  pub line_type: String,
  pub old_line_number: Option<u32>,
  pub new_line_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunk {
  pub header: String,
  pub old_start: u32,
  pub old_lines: u32,
  pub new_start: u32,
  pub new_lines: u32,
  pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffWithHunks {
  pub path: String,
  pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, uniffi::Enum)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, uniffi::Enum)]
#[serde(rename_all = "camelCase")]
pub enum ResourceGroupKind {
  Index,
  WorkingTree,
  Untracked,
  Merge,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct FileEntry {
  pub path: String,
  pub status: FileStatus,
  pub rename_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ResourceGroup {
  pub kind: ResourceGroupKind,
  pub label: String,
  pub files: Vec<FileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, uniffi::Enum)]
#[serde(rename_all = "camelCase")]
pub enum RepoOperationState {
  #[serde(rename = "none")]
  Clean,
  Merging,
  Rebasing,
  CherryPicking,
  Reverting,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct RepositoryStatus {
  pub root: String,
  pub head_branch: Option<String>,
  pub head_commit: Option<String>,
  pub ahead: u32,
  pub behind: u32,
  pub groups: Vec<ResourceGroup>,
  pub operation_state: RepoOperationState,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct DiffContent {
  pub path: String,
  pub original: String,
  pub modified: String,
  pub original_language: Option<String>,
  pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct BranchEntry {
  pub name: String,
  pub is_head: bool,
  pub is_remote: bool,
  pub upstream: Option<String>,
  pub ahead: u32,
  pub behind: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
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

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileEntry {
  pub path: String,
  pub status: String,
  pub old_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct CommitDetail {
  pub commit: CommitEntry,
  pub files: Vec<CommitFileEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct CommitDiffContent {
  pub path: String,
  pub original: String,
  pub modified: String,
  pub language: Option<String>,
  pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct StashEntry {
  pub index: u32,
  pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct TagEntry {
  pub name: String,
  pub message: Option<String>,
  pub target_id: String,
  pub is_annotated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct BlameLineGroup {
  pub commit_id: String,
  pub short_id: String,
  pub author_name: String,
  pub author_email: String,
  pub author_date: String,
  pub summary: String,
  pub start_line: u32,
  pub end_line: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct FileBlame {
  pub path: String,
  pub line_groups: Vec<BlameLineGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct LastCommitInfo {
  pub short_id: String,
  pub message: String,
  pub author_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ProjectInfo {
  pub path: String,
  pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ExplorerEntry {
  pub name: String,
  pub path: String,
  pub is_directory: bool,
  pub is_symlink: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct FileContent {
  pub path: String,
  pub content: String,
  pub language: Option<String>,
  pub file_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct FuzzyFileResult {
  pub path: String,
  pub score: u32,
  pub match_positions: Vec<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, uniffi::Record)]
#[serde(rename_all = "camelCase")]
pub struct ContentSearchResult {
  pub path: String,
  pub line_number: u32,
  pub column: u32,
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
      operation_state: RepoOperationState::Clean,
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
}
