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
  status: FileStatus;
  oldPath: string | null;
}

export interface CommitDetail {
  commit: CommitEntry;
  files: CommitFileEntry[];
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
  ignored: boolean;
}

export interface FileContent {
  path: string;
  content: string;
  language: string | null;
  fileType: string;
  contentHash: string;
}

export type WriteFileResult = {
  contentHash: string;
};

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

export type SyncKind = "fetch" | "pull" | "push" | "pullThenPush";

export type FileSelection = {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
};

export type SessionRepo = {
  root: string;
  headBranch: string | null;
  headCommit: string | null;
  ahead: number;
  behind: number;
  operationState: RepoOperationState;
  phase: StatusPhase;
};

export type SessionSelection = {
  file: FileSelection | null;
  commit: string | null;
};

export type SessionScm = {
  amendMode: boolean;
  commitMessage: string;
  fileFilter: string;
};

export type SyncAction = {
  enabled: boolean;
  kind: SyncKind;
  destructive: boolean;
};

export type OperationActions = {
  continue: boolean;
  abort: boolean;
  skip: boolean;
  abortDestructive: boolean;
};

export type SessionActions = {
  canCommit: boolean;
  commitLabel: string;
  commitDestructive: boolean;
  canStageAll: boolean;
  canUnstageAll: boolean;
  canDiscardAll: boolean;
  discardAllDestructive: boolean;
  sync: SyncAction;
  operation: OperationActions;
};

export type SessionSnapshot = {
  sessionGeneration: number;
  sessionRevision: number;
  statusGeneration: number;
  statusRevision: number;
  repo: SessionRepo;
  groups: ResourceGroup[];
  selection: SessionSelection;
  scm: SessionScm;
  actions: SessionActions;
  lastCommit: LastCommitInfo | null;
  branches: BranchEntry[];
  stashes: StashEntry[];
  tags: TagEntry[];
  commitLog: CommitEntry[];
  commitDetail: CommitDetail | null;
  fileHistoryPath: string | null;
  error: string | null;
};

export type SessionStatusExtras = {
  lastCommit?: LastCommitInfo | null;
  branches?: BranchEntry[];
  tags?: TagEntry[];
  commitLog?: CommitEntry[];
  stashes?: StashEntry[];
};

export type SessionStatusEvent = {
  sessionGeneration: number;
  sessionRevision: number;
  statusGeneration: number;
  statusRevision: number;
  repo: SessionRepo;
  groups: ResourceGroup[];
  actions: SessionActions;
  selection: SessionSelection;
  extras?: SessionStatusExtras;
};

export type SessionPatch =
  | { kind: "scm"; scm: SessionScm; actions: SessionActions }
  | { kind: "actions"; actions: SessionActions }
  | { kind: "fileHistory"; path: string | null; commitLog: CommitEntry[] }
  | { kind: "commitLog"; commitLog: CommitEntry[] }
  | { kind: "commit"; id: string | null; detail: CommitDetail | null };

export type DiffPresence = {
  oldExists: boolean;
  newExists: boolean;
};

export type DiffHunkPayload = DiffHunk & { id: string };

export type DiffPayload = {
  path: string;
  original: string;
  modified: string;
  language: string | null;
  fileType: string;
  hunks: DiffHunkPayload[];
  presence: DiffPresence;
  editable: boolean;
  enableLineSelection: boolean;
  staged: boolean;
  contentHash: string;
};

export type Intent =
  | { type: "openRepository"; path: string }
  | { type: "cloneRepository"; url: string; directory: string }
  | { type: "refreshStatus" }
  | { type: "clearFile" }
  | { type: "setAmend"; enabled: boolean }
  | { type: "setCommitMessage"; message: string }
  | { type: "setFileFilter"; filter: string }
  | { type: "stage"; paths: string[] }
  | { type: "stageAll" }
  | { type: "unstage"; paths: string[] }
  | { type: "unstageAll" }
  | { type: "discard"; paths: string[]; confirmed: boolean }
  | { type: "commit"; confirmed: boolean }
  | { type: "commitAndPush"; confirmed: boolean }
  | { type: "commitAndSync"; confirmed: boolean }
  | { type: "sync" }
  | { type: "push"; force: boolean; confirmed: boolean }
  | { type: "pull"; rebase: boolean }
  | { type: "fetch"; prune: boolean }
  | { type: "undoCommit"; confirmed: boolean }
  | { type: "operationContinue" }
  | { type: "operationAbort" }
  | { type: "operationSkip" }
  | { type: "stageHunk"; hunkId: string }
  | { type: "unstageHunk"; hunkId: string }
  | { type: "discardHunk"; hunkId: string; confirmed: boolean }
  | { type: "stageLines"; path: string; start: number; end: number; staged: boolean }
  | { type: "openScmDiff"; path: string; staged: boolean; groupKind?: ResourceGroupKind }
  | { type: "openCommitDiff"; commit: string; path: string }
  | { type: "openBlame"; path: string }
  | { type: "resolveConflict"; path: string; contents: string }
  | { type: "stashSave"; includeUntracked: boolean; stagedOnly: boolean; message: string | null }
  | { type: "stashApply"; index: number }
  | { type: "stashPop"; index: number }
  | { type: "stashDrop"; index: number; confirmed: boolean }
  | { type: "checkoutBranch"; name: string }
  | { type: "createBranch"; name: string; from: string | null }
  | { type: "deleteBranch"; name: string; force: boolean; confirmed: boolean }
  | { type: "renameBranch"; oldName: string; newName: string }
  | { type: "mergeBranch"; name: string }
  | { type: "rebaseBranch"; name: string }
  | { type: "deleteRemoteBranch"; name: string }
  | { type: "createTag"; name: string; message: string | null; commit: string | null }
  | { type: "deleteTag"; name: string; confirmed: boolean }
  | { type: "pushTag"; name: string }
  | { type: "deleteRemoteTag"; name: string }
  | { type: "cherryPick"; commit: string }
  | { type: "reset"; commit: string; mode: string; confirmed: boolean }
  | { type: "loadMoreLog" }
  | { type: "openFileHistory"; path: string }
  | { type: "clearFileHistory" }
  | { type: "selectCommit"; id: string }
  | { type: "deleteFile"; path: string; confirmed: boolean }
  | { type: "addToGitignore"; path: string };

export type IntentOutcome =
  | { kind: "ack"; sessionGeneration?: number; sessionRevision?: number }
  | { kind: "patch"; patch: SessionPatch; sessionGeneration: number; sessionRevision: number }
  | { kind: "snapshot"; snapshot: SessionSnapshot }
  | { kind: "diff"; payload: DiffPayload; sessionGeneration: number; sessionRevision: number }
  | { kind: "blame"; payload: FileBlame; sessionGeneration: number; sessionRevision: number }
  | { kind: "needsConfirmation"; action: string; message: string };
