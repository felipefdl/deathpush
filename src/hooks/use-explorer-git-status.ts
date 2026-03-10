import { useMemo } from "react";
import type { FileStatus, RepositoryStatus } from "../lib/git-types";
import { getStatusColor } from "../lib/status-colors";
import { getStatusLabel } from "../lib/status-icons";
import { useRepositoryStore } from "../stores/repository-store";

export interface GitDecoration {
  status: FileStatus;
  color: string;
  label: string;
}

export interface GitDecorationMaps {
  fileMap: Map<string, GitDecoration>;
  dirMap: Map<string, GitDecoration>;
}

const STATUS_PRIORITY: Record<FileStatus, number> = {
  bothDeleted: 10,
  addedByUs: 10,
  deletedByThem: 10,
  addedByThem: 10,
  deletedByUs: 10,
  bothAdded: 10,
  bothModified: 10,
  deleted: 9,
  indexDeleted: 9,
  modified: 8,
  indexModified: 8,
  typeChanged: 8,
  added: 7,
  indexAdded: 7,
  intentToAdd: 7,
  renamed: 6,
  indexRenamed: 6,
  intentToRename: 6,
  copied: 5,
  indexCopied: 5,
  untracked: 4,
  ignored: 0,
};

const buildMaps = (status: RepositoryStatus | null): GitDecorationMaps => {
  const fileMap = new Map<string, GitDecoration>();
  const dirMap = new Map<string, GitDecoration>();

  if (!status) return { fileMap, dirMap };

  for (const group of status.groups) {
    for (const file of group.files) {
      const priority = STATUS_PRIORITY[file.status] ?? 0;
      const existing = fileMap.get(file.path);
      if (!existing || priority > (STATUS_PRIORITY[existing.status] ?? 0)) {
        fileMap.set(file.path, {
          status: file.status,
          color: getStatusColor(file.status),
          label: getStatusLabel(file.status),
        });
      }
    }
  }

  // Propagate to parent directories (skip ignored files)
  for (const [filePath, decoration] of fileMap) {
    if (decoration.status === "ignored") continue;
    const priority = STATUS_PRIORITY[decoration.status] ?? 0;
    const parts = filePath.split("/");
    for (let i = 1; i < parts.length; i++) {
      const dirPath = parts.slice(0, i).join("/");
      const existing = dirMap.get(dirPath);
      if (!existing || priority > (STATUS_PRIORITY[existing.status] ?? 0)) {
        dirMap.set(dirPath, decoration);
      }
    }
  }

  return { fileMap, dirMap };
};

export const useExplorerGitStatus = (): GitDecorationMaps => {
  const status = useRepositoryStore((s) => s.status);
  return useMemo(() => buildMaps(status), [status]);
};
