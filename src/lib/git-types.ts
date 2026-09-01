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

export type RepositoryIdentity = {
  root: string;
  headBranch: string | null;
};

export type StatusPhase = "scanning" | "settled" | "storm";

export type StatusKey = {
  group: ResourceGroupKind;
  path: string;
};

export type StatusEntry = {
  group: ResourceGroupKind;
  path: string;
  status: FileStatus;
  renamePath: string | null;
};

export type RepositoryMetadata = {
  root: string;
  headBranch: string | null;
  headCommit: string | null;
  ahead: number;
  behind: number;
  operationState: RepoOperationState;
};

export type StatusPatch = {
  generation: number;
  baseRevision: number;
  revision: number;
  upserts: StatusEntry[];
  removals: StatusKey[];
  metadata?: RepositoryMetadata;
  phase: StatusPhase;
};

export type StatusSnapshot = {
  generation: number;
  revision: number;
  phase: StatusPhase;
  entries: StatusEntry[];
  metadata: RepositoryMetadata;
};

export type PathChangeKind = "content" | "git" | "structural";

export type PathChangeScope = "exact" | "subtree" | "repository";

export type PathsChanged = {
  paths: string[];
  kind: PathChangeKind;
  scope: PathChangeScope;
  generation: number;
  storm: boolean;
};

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
  avatarUrl: string;
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

export interface ExplorerEntry {
  name: string;
  path: string;
  isDirectory: boolean;
  isSymlink: boolean;
}

export interface FileContent {
  path: string;
  content: string;
  language: string | null;
  fileType: string;
}

export interface FuzzyFileResult {
  path: string;
  score: number;
  matchPositions: number[];
}

export interface ContentSearchResult {
  path: string;
  lineNumber: number;
  column: number;
  lineContent: string;
}
