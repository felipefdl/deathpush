import { invoke } from "@tauri-apps/api/core";
import type {
  BranchEntry,
  CommitDetail,
  CommitDiffContent,
  CommitEntry,
  DiffContent,
  FileBlame,
  FileDiffWithHunks,
  LastCommitInfo,
  RepositoryStatus,
  StashEntry,
  TagEntry,
} from "./git-types";

export const openRepository = (path: string): Promise<RepositoryStatus> =>
  invoke("open_repository", { path });

export const getStatus = (): Promise<RepositoryStatus> =>
  invoke("get_status");

export const getFileDiff = (path: string, staged: boolean): Promise<DiffContent> =>
  invoke("get_file_diff", { path, staged });

export const stageFiles = (paths: string[]): Promise<RepositoryStatus> =>
  invoke("stage_files", { paths });

export const stageAll = (): Promise<RepositoryStatus> =>
  invoke("stage_all");

export const unstageFiles = (paths: string[]): Promise<RepositoryStatus> =>
  invoke("unstage_files", { paths });

export const unstageAll = (): Promise<RepositoryStatus> =>
  invoke("unstage_all");

export const discardChanges = (paths: string[]): Promise<RepositoryStatus> =>
  invoke("discard_changes", { paths });

export const commitChanges = (message: string, amend: boolean = false): Promise<RepositoryStatus> =>
  invoke("commit", { message, amend });

export const listBranches = (): Promise<BranchEntry[]> =>
  invoke("list_branches");

export const checkoutBranch = (name: string): Promise<RepositoryStatus> =>
  invoke("checkout_branch", { name });

export const createBranch = (name: string, from?: string): Promise<RepositoryStatus> =>
  invoke("create_branch", { name, from: from ?? null });

export const deleteBranch = (name: string, force: boolean = false): Promise<void> =>
  invoke("delete_branch", { name, force });

export const push = (remote: string = "origin", branch: string = "", force: boolean = false): Promise<void> =>
  invoke("push", { remote, branch, force });

export const pull = (remote: string = "origin", branch: string = "", rebase: boolean = false): Promise<void> =>
  invoke("pull", { remote, branch, rebase });

export const fetchRemote = (remote: string = "origin", prune: boolean = false): Promise<void> =>
  invoke("fetch", { remote, prune });

export const getLastCommitMessage = (): Promise<string> =>
  invoke("get_last_commit_message");

export const undoLastCommit = (): Promise<RepositoryStatus> =>
  invoke("undo_last_commit");

export const stashSave = (message?: string): Promise<RepositoryStatus> =>
  invoke("stash_save", { message: message ?? null });

export const stashList = (): Promise<StashEntry[]> =>
  invoke("stash_list");

export const stashApply = (index: number): Promise<RepositoryStatus> =>
  invoke("stash_apply", { index });

export const stashPop = (index: number): Promise<RepositoryStatus> =>
  invoke("stash_pop", { index });

export const stashDrop = (index: number): Promise<StashEntry[]> =>
  invoke("stash_drop", { index });

export const getCommitLog = (skip: number, limit: number): Promise<CommitEntry[]> =>
  invoke("get_commit_log", { skip, limit });

export const getCommitDetail = (id: string): Promise<CommitDetail> =>
  invoke("get_commit_detail", { id });

export const getCommitFileDiff = (commitId: string, path: string): Promise<CommitDiffContent> =>
  invoke("get_commit_file_diff", { commitId, path });

export const listTags = (): Promise<TagEntry[]> =>
  invoke("list_tags");

export const createTag = (name: string, message?: string, commit?: string): Promise<TagEntry[]> =>
  invoke("create_tag", { name, message: message ?? null, commit: commit ?? null });

export const deleteTag = (name: string): Promise<TagEntry[]> =>
  invoke("delete_tag", { name });

export const pushTag = (remote: string, tag: string): Promise<void> =>
  invoke("push_tag", { remote, tag });

export const writeFile = (path: string, content: string): Promise<void> =>
  invoke("write_file", { path, content });

export const deleteFile = (path: string): Promise<RepositoryStatus> =>
  invoke("delete_file", { path });

export const openInEditor = (path: string): Promise<void> =>
  invoke("open_in_editor", { path });

export const revealInFileManager = (path: string): Promise<void> =>
  invoke("reveal_in_file_manager", { path });

export const addToGitignore = (pattern: string): Promise<RepositoryStatus> =>
  invoke("add_to_gitignore", { pattern });

export const getFileHunks = (path: string, staged: boolean): Promise<FileDiffWithHunks> =>
  invoke("get_file_hunks", { path, staged });

export const stageHunk = (path: string, hunkIndex: number, staged: boolean): Promise<RepositoryStatus> =>
  invoke("stage_hunk", { path, hunkIndex, staged });

export const cloneRepository = (url: string, path: string): Promise<RepositoryStatus> =>
  invoke("clone_repository", { url, path });

export const mergeContinue = (): Promise<RepositoryStatus> =>
  invoke("merge_continue");

export const mergeAbort = (): Promise<RepositoryStatus> =>
  invoke("merge_abort");

export const rebaseContinue = (): Promise<RepositoryStatus> =>
  invoke("rebase_continue");

export const rebaseAbort = (): Promise<RepositoryStatus> =>
  invoke("rebase_abort");

export const rebaseSkip = (): Promise<RepositoryStatus> =>
  invoke("rebase_skip");

export const cherryPick = (commitId: string): Promise<RepositoryStatus> =>
  invoke("cherry_pick", { commitId });

export const resetToCommit = (id: string, mode: string): Promise<RepositoryStatus> =>
  invoke("reset_to_commit", { id, mode });

export const getGitConfig = (key: string): Promise<string> =>
  invoke("get_git_config", { key });

export const setGitConfig = (key: string, value: string): Promise<void> =>
  invoke("set_git_config", { key, value });

export const getFileBlame = (path: string): Promise<FileBlame> =>
  invoke("get_file_blame", { path });

export const getFileLog = (path: string, skip: number, limit: number): Promise<CommitEntry[]> =>
  invoke("get_file_log", { path, skip, limit });

export const getLastCommitInfo = (): Promise<LastCommitInfo> =>
  invoke("get_last_commit_info");

export const newWindow = (): Promise<void> =>
  invoke("new_window");

export interface ProjectInfo {
  path: string;
  name: string;
}

export const getInitialPath = (): Promise<string | null> =>
  invoke("get_initial_path");

export const scanProjectsDirectory = (path: string, depth: number): Promise<ProjectInfo[]> =>
  invoke("scan_projects_directory", { path, depth });

export interface CliInstallStatus {
  installed: boolean;
  dpPath: string | null;
  deathpushPath: string | null;
}

export const checkCliInstalled = (): Promise<CliInstallStatus> =>
  invoke("check_cli_installed");

export const installCli = (): Promise<void> =>
  invoke("install_cli");

export const uninstallCli = (): Promise<void> =>
  invoke("uninstall_cli");
