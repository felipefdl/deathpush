export type FileStatus =
  | "modified"
  | "added"
  | "deleted"
  | "renamed"
  | "copied"
  | "untracked"
  | "ignored"
  | "typeChanged"
  | "indexModified"
  | "indexAdded"
  | "indexDeleted"
  | "indexRenamed"
  | "indexCopied"
  | "intentToAdd"
  | "intentToRename"
  | "bothDeleted"
  | "addedByUs"
  | "deletedByThem"
  | "addedByThem"
  | "deletedByUs"
  | "bothAdded"
  | "bothModified";

export type ResourceGroupKind = "index" | "workingTree" | "untracked" | "merge";

export interface FileEntry {
  path: string;
  status: FileStatus;
  renamePath: string | null;
}

export interface ResourceGroup {
  kind: ResourceGroupKind;
  label: string;
  files: FileEntry[];
}

export type RepoOperationState = "none" | "merging" | "rebasing" | "cherryPicking" | "reverting";

export interface RepositoryStatus {
  root: string;
  headBranch: string | null;
  headCommit: string | null;
  ahead: number;
  behind: number;
  groups: ResourceGroup[];
  operationState: RepoOperationState;
}

export interface DiffContent {
  path: string;
  original: string;
  modified: string;
  originalLanguage: string | null;
  fileType: string;
}

export interface BranchEntry {
  name: string;
  isHead: boolean;
  isRemote: boolean;
  upstream: string | null;
  ahead: number;
  behind: number;
}

export interface StashEntry {
  index: number;
  message: string;
}

export interface TagEntry {
  name: string;
  message: string | null;
  targetId: string;
  isAnnotated: boolean;
}

export interface CommitEntry {
  id: string;
  shortId: string;
  message: string;
  authorName: string;
  authorEmail: string;
  authorDate: string;
  parentIds: string[];
}

export interface CommitFileEntry {
  path: string;
  status: string;
  oldPath: string | null;
}

export interface CommitDetail {
  commit: CommitEntry;
  files: CommitFileEntry[];
}

export interface CommitDiffContent {
  path: string;
  original: string;
  modified: string;
  language: string | null;
  fileType: string;
}

export interface DiffLine {
  content: string;
  lineType: string;
  oldLineNumber: number | null;
  newLineNumber: number | null;
}

export interface DiffHunk {
  header: string;
  oldStart: number;
  oldLines: number;
  newStart: number;
  newLines: number;
  lines: DiffLine[];
}

export interface FileDiffWithHunks {
  path: string;
  hunks: DiffHunk[];
}

export interface BlameLineGroup {
  commitId: string;
  shortId: string;
  authorName: string;
  authorEmail: string;
  authorDate: string;
  summary: string;
  startLine: number;
  endLine: number;
}

export interface FileBlame {
  path: string;
  lineGroups: BlameLineGroup[];
}

export interface LastCommitInfo {
  shortId: string;
  message: string;
  authorDate: string;
}
