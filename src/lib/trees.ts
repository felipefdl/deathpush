import type { GitStatus, GitStatusEntry } from "@pierre/trees";
import type { ExplorerEntry, FileEntry, FileStatus } from "./git-types";

const TREE_STATUS_BY_FILE_STATUS: Record<FileStatus, GitStatus> = {
  modified: "modified",
  added: "added",
  deleted: "deleted",
  renamed: "renamed",
  copied: "renamed",
  untracked: "untracked",
  ignored: "ignored",
  typeChanged: "modified",
  indexModified: "modified",
  indexAdded: "added",
  indexDeleted: "deleted",
  indexRenamed: "renamed",
  indexCopied: "renamed",
  intentToAdd: "added",
  intentToRename: "renamed",
  bothDeleted: "deleted",
  addedByUs: "added",
  deletedByThem: "deleted",
  addedByThem: "added",
  deletedByUs: "deleted",
  bothAdded: "added",
  bothModified: "modified",
};

const TREE_STATUS_PRIORITY: Record<GitStatus, number> = {
  deleted: 5,
  modified: 4,
  added: 3,
  renamed: 2,
  untracked: 1,
  ignored: 0,
};

export const explorerEntriesToTreePaths = (entries: readonly ExplorerEntry[]): string[] =>
  entries.map((entry) => (entry.isDirectory ? `${entry.path}/` : entry.path));

export const sameTreePaths = (left: readonly string[], right: readonly string[]): boolean => {
  if (left.length !== right.length) return false;
  const other = new Set(right);
  for (const path of left) if (!other.has(path)) return false;
  return true;
};

export const directoryNeedsChildren = (entries: readonly ExplorerEntry[], directoryPath: string): boolean => {
  const directory = directoryPath.endsWith("/") ? directoryPath.slice(0, -1) : directoryPath;
  if (!directory) return false;
  const prefix = `${directory}/`;
  return !entries.some((entry) => entry.path.startsWith(prefix));
};

export const explorerGitStatus = (entries: readonly ExplorerEntry[], files: readonly FileEntry[]): GitStatusEntry[] => {
  const ignored: FileEntry[] = entries
    .filter((entry) => entry.ignored)
    .map((entry) => ({
      path: entry.isDirectory ? `${entry.path}/` : entry.path,
      status: "ignored",
      renamePath: null,
    }));
  return fileEntriesToTreeGitStatus([...ignored, ...files]);
};

export const fileEntriesToTreeGitStatus = (files: readonly FileEntry[]): GitStatusEntry[] => {
  const statusByPath = new Map<string, GitStatus>();
  for (const file of files) {
    const status = TREE_STATUS_BY_FILE_STATUS[file.status];
    const existing = statusByPath.get(file.path);
    if (!existing || TREE_STATUS_PRIORITY[status] > TREE_STATUS_PRIORITY[existing]) {
      statusByPath.set(file.path, status);
    }
  }
  return [...statusByPath].map(([path, status]) => ({ path, status }));
};
