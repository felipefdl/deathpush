import { describe, expect, it } from "vite-plus/test";
import type { ExplorerEntry, FileEntry } from "./git-types";
import { explorerEntriesToTreePaths, fileEntriesToTreeGitStatus, sameTreePaths } from "./trees";

describe("tree adapters", () => {
  it("marks explicit directories with the Trees trailing slash contract", () => {
    const entries: ExplorerEntry[] = [
      { name: "empty", path: "empty", isDirectory: true, isSymlink: false },
      { name: "index.ts", path: "src/index.ts", isDirectory: false, isSymlink: false },
    ];

    expect(explorerEntriesToTreePaths(entries)).toEqual(["empty/", "src/index.ts"]);
  });

  it("maps DeathPush file states to Trees Git states", () => {
    const files: FileEntry[] = [
      { path: "modified.ts", status: "indexModified", renamePath: null },
      { path: "added.ts", status: "intentToAdd", renamePath: null },
      { path: "deleted.ts", status: "deletedByThem", renamePath: null },
      { path: "renamed.ts", status: "indexCopied", renamePath: null },
      { path: "untracked.ts", status: "untracked", renamePath: null },
      { path: "ignored.ts", status: "ignored", renamePath: null },
    ];

    expect(fileEntriesToTreeGitStatus(files)).toEqual([
      { path: "modified.ts", status: "modified" },
      { path: "added.ts", status: "added" },
      { path: "deleted.ts", status: "deleted" },
      { path: "renamed.ts", status: "renamed" },
      { path: "untracked.ts", status: "untracked" },
      { path: "ignored.ts", status: "ignored" },
    ]);
  });

  it("treats the same path set as unchanged even when order differs", () => {
    expect(sameTreePaths(["src/", "README.md"], ["README.md", "src/"])).toBe(true);
    expect(sameTreePaths(["src/", "README.md"], ["src/", "vite.config.ts"])).toBe(false);
  });
});
