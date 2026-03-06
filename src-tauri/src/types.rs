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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileDiffWithHunks {
  pub path: String,
  pub hunks: Vec<DiffHunk>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitFileEntry {
  pub path: String,
  pub status: String,
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
