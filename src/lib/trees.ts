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
