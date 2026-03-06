import type { FileStatus } from "./git-types";

const STATUS_COLORS: Record<FileStatus, string> = {
  modified: "var(--vscode-gitDecoration-modifiedResourceForeground)",
  added: "var(--vscode-gitDecoration-addedResourceForeground)",
  deleted: "var(--vscode-gitDecoration-deletedResourceForeground)",
  renamed: "var(--vscode-gitDecoration-renamedResourceForeground)",
  copied: "var(--vscode-gitDecoration-addedResourceForeground)",
  untracked: "var(--vscode-gitDecoration-untrackedResourceForeground)",
  ignored: "var(--vscode-gitDecoration-ignoredResourceForeground)",
  typeChanged: "var(--vscode-gitDecoration-modifiedResourceForeground)",
  indexModified: "var(--vscode-gitDecoration-stageModifiedResourceForeground)",
  indexAdded: "var(--vscode-gitDecoration-addedResourceForeground)",
  indexDeleted: "var(--vscode-gitDecoration-stageDeletedResourceForeground)",
  indexRenamed: "var(--vscode-gitDecoration-renamedResourceForeground)",
  indexCopied: "var(--vscode-gitDecoration-addedResourceForeground)",
  intentToAdd: "var(--vscode-gitDecoration-addedResourceForeground)",
  intentToRename: "var(--vscode-gitDecoration-renamedResourceForeground)",
  bothDeleted: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  addedByUs: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  deletedByThem: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  addedByThem: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  deletedByUs: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  bothAdded: "var(--vscode-gitDecoration-conflictingResourceForeground)",
  bothModified: "var(--vscode-gitDecoration-conflictingResourceForeground)",
};

export const getStatusColor = (status: FileStatus): string =>
  STATUS_COLORS[status] ?? "var(--vscode-editor-foreground)";
