import { describe, it, expect } from "vitest";
import { getStatusColor } from "./status-colors";
import type { FileStatus } from "./git-types";

describe("getStatusColor", () => {
  it("returns modified color for modified status", () => {
    expect(getStatusColor("modified")).toBe("var(--vscode-gitDecoration-modifiedResourceForeground)");
  });

  it("returns deleted color for deleted status", () => {
    expect(getStatusColor("deleted")).toBe("var(--vscode-gitDecoration-deletedResourceForeground)");
  });

  it("returns untracked color for untracked status", () => {
    expect(getStatusColor("untracked")).toBe("var(--vscode-gitDecoration-untrackedResourceForeground)");
  });

  it("returns conflict color for merge statuses", () => {
    const mergeStatuses: FileStatus[] = ["bothDeleted", "addedByUs", "deletedByThem", "addedByThem", "deletedByUs", "bothAdded", "bothModified"];
    for (const status of mergeStatuses) {
      expect(getStatusColor(status)).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
    }
  });

  it("returns added color for index added", () => {
    expect(getStatusColor("indexAdded")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns renamed color for renamed status", () => {
    expect(getStatusColor("renamed")).toBe("var(--vscode-gitDecoration-renamedResourceForeground)");
  });
});
