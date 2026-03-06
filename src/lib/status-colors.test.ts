import { describe, it, expect } from "vitest";
import { getStatusColor } from "./status-colors";

describe("getStatusColor", () => {
  it("returns modified color for modified", () => {
    expect(getStatusColor("modified")).toBe("var(--vscode-gitDecoration-modifiedResourceForeground)");
  });

  it("returns added color for added", () => {
    expect(getStatusColor("added")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns deleted color for deleted", () => {
    expect(getStatusColor("deleted")).toBe("var(--vscode-gitDecoration-deletedResourceForeground)");
  });

  it("returns renamed color for renamed", () => {
    expect(getStatusColor("renamed")).toBe("var(--vscode-gitDecoration-renamedResourceForeground)");
  });

  it("returns added color for copied", () => {
    expect(getStatusColor("copied")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns untracked color for untracked", () => {
    expect(getStatusColor("untracked")).toBe("var(--vscode-gitDecoration-untrackedResourceForeground)");
  });

  it("returns ignored color for ignored", () => {
    expect(getStatusColor("ignored")).toBe("var(--vscode-gitDecoration-ignoredResourceForeground)");
  });

  it("returns modified color for typeChanged", () => {
    expect(getStatusColor("typeChanged")).toBe("var(--vscode-gitDecoration-modifiedResourceForeground)");
  });

  it("returns stageModified color for indexModified", () => {
    expect(getStatusColor("indexModified")).toBe("var(--vscode-gitDecoration-stageModifiedResourceForeground)");
  });

  it("returns added color for indexAdded", () => {
    expect(getStatusColor("indexAdded")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns stageDeleted color for indexDeleted", () => {
    expect(getStatusColor("indexDeleted")).toBe("var(--vscode-gitDecoration-stageDeletedResourceForeground)");
  });

  it("returns renamed color for indexRenamed", () => {
    expect(getStatusColor("indexRenamed")).toBe("var(--vscode-gitDecoration-renamedResourceForeground)");
  });

  it("returns added color for indexCopied", () => {
    expect(getStatusColor("indexCopied")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns added color for intentToAdd", () => {
    expect(getStatusColor("intentToAdd")).toBe("var(--vscode-gitDecoration-addedResourceForeground)");
  });

  it("returns renamed color for intentToRename", () => {
    expect(getStatusColor("intentToRename")).toBe("var(--vscode-gitDecoration-renamedResourceForeground)");
  });

  it("returns conflicting color for bothDeleted", () => {
    expect(getStatusColor("bothDeleted")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for addedByUs", () => {
    expect(getStatusColor("addedByUs")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for deletedByThem", () => {
    expect(getStatusColor("deletedByThem")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for addedByThem", () => {
    expect(getStatusColor("addedByThem")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for deletedByUs", () => {
    expect(getStatusColor("deletedByUs")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for bothAdded", () => {
    expect(getStatusColor("bothAdded")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });

  it("returns conflicting color for bothModified", () => {
    expect(getStatusColor("bothModified")).toBe("var(--vscode-gitDecoration-conflictingResourceForeground)");
  });
});
