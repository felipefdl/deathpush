import { invoke } from "@tauri-apps/api/core";
import type {
  ContentSearchResult,
  ExplorerEntry,
  FileContent,
  FuzzyFileResult,
  Intent,
  IntentOutcome,
  SessionSnapshot,
  WriteFileResult,
} from "./git-types";

export const writeFile = (path: string, content: string): Promise<WriteFileResult> =>
  invoke("write_file", { path, content });

export const openInEditor = (path: string): Promise<void> => invoke("open_in_editor", { path });

export const revealInFileManager = (path: string): Promise<void> => invoke("reveal_in_file_manager", { path });

export const getGitConfig = (key: string): Promise<string> => invoke("get_git_config", { key });

export const setGitConfig = (key: string, value: string): Promise<void> => invoke("set_git_config", { key, value });

export const newWindow = (path?: string): Promise<void> => invoke("new_window", { path: path ?? null });

export interface ProjectInfo {
  path: string;
  name: string;
}

export const getInitialPath = (): Promise<string | null> => invoke("get_initial_path");

export const scanWorkspaceProjects = (entries: { directory: string; depth: number }[]): Promise<ProjectInfo[]> =>
  invoke("scan_workspace_projects", { entries });

export type NestedRepository = {
  path: string;
  name: string;
  branch: string | null;
};

export type WorktreeInfo = {
  path: string;
  name: string;
  branch: string | null;
  isMain: boolean;
};

export const discoverNestedRepositories = (): Promise<NestedRepository[]> => invoke("discover_nested_repositories");

export const detectWorktrees = (): Promise<WorktreeInfo[]> => invoke("detect_worktrees");

export interface CliInstallStatus {
  installed: boolean;
  dpPath: string | null;
  deathpushPath: string | null;
}

export const checkCliInstalled = (): Promise<CliInstallStatus> => invoke("check_cli_installed");

export const installCli = (): Promise<void> => invoke("install_cli");

export const uninstallCli = (): Promise<void> => invoke("uninstall_cli");

export const setRepoMenuEnabled = (enabled: boolean): Promise<void> => invoke("set_repo_menu_enabled", { enabled });

export const setNativeTheme = (dark: boolean): Promise<void> => invoke("set_native_theme", { dark });

export const windowConfirmClose = (): Promise<void> => invoke("window_confirm_close");

export const terminalsHaveActiveProcess = (): Promise<boolean> => invoke("terminals_have_active_process");

export const listRepositoryTree = (): Promise<ExplorerEntry[]> => invoke("list_repository_tree");

export const listRepositoryChildren = (path: string): Promise<ExplorerEntry[]> =>
  invoke("list_repository_children", { path });

export const readFileContent = (path: string): Promise<FileContent> => invoke("read_file_content", { path });

export const renameEntry = (oldPath: string, newName: string): Promise<void> =>
  invoke("rename_entry", { oldPath, newName });

export const createDirectory = (path: string): Promise<void> => invoke("create_directory", { path });

export type ConflictResolution = "error" | "replace" | "keep-both";

export const copyEntries = (
  sources: string[],
  destinationDir: string,
  onConflict?: ConflictResolution
): Promise<void> => invoke("copy_entries", { sources, destinationDir, onConflict: onConflict ?? null });

export const moveEntries = (
  sources: string[],
  destinationDir: string,
  onConflict?: ConflictResolution
): Promise<void> => invoke("move_entries", { sources, destinationDir, onConflict: onConflict ?? null });

export const duplicateEntry = (path: string): Promise<string> => invoke("duplicate_entry", { path });

export const importFiles = (
  sources: string[],
  destinationDir: string,
  onConflict?: ConflictResolution
): Promise<void> => invoke("import_files", { sources, destinationDir, onConflict: onConflict ?? null });

export const fuzzyFindFiles = (query: string, maxResults: number): Promise<FuzzyFileResult[]> =>
  invoke("fuzzy_find_files", { query, maxResults });

export const searchFileContents = (query: string, maxResults: number): Promise<ContentSearchResult[]> =>
  invoke("search_file_contents", { query, maxResults });

export const getSessionSnapshot = (): Promise<SessionSnapshot> => invoke("get_session_snapshot");

export const sessionIntent = (intent: Intent): Promise<IntentOutcome> => invoke("session_intent", { intent });
