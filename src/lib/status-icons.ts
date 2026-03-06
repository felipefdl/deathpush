import type { FileStatus } from "./git-types";

const STATUS_LABELS: Record<FileStatus, string> = {
  modified: "M",
  added: "A",
  deleted: "D",
  renamed: "R",
  copied: "C",
  untracked: "U",
  ignored: "!",
  typeChanged: "T",
  indexModified: "M",
  indexAdded: "A",
  indexDeleted: "D",
  indexRenamed: "R",
  indexCopied: "C",
  intentToAdd: "A",
  intentToRename: "R",
  bothDeleted: "!",
  addedByUs: "!",
  deletedByThem: "!",
  addedByThem: "!",
  deletedByUs: "!",
  bothAdded: "!",
  bothModified: "!",
};

export const getStatusLabel = (status: FileStatus): string =>
  STATUS_LABELS[status] ?? "?";
