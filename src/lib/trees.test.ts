import { describe, expect, it } from "vite-plus/test";
import type { ExplorerEntry, FileEntry } from "./git-types";
import {
  directoryNeedsChildren,
  explorerEntriesToTreePaths,
  explorerGitStatus,
  fileEntriesToTreeGitStatus,
  sameTreePaths,
} from "./trees";

describe("tree adapters", () => {
  it("marks explicit directories with the Trees trailing slash contract", () => {
    const entries: ExplorerEntry[] = [
      { name: "empty", path: "empty", isDirectory: true, isSymlink: false, ignored: false },
      { name: "index.ts", path: "src/index.ts", isDirectory: false, isSymlink: false, ignored: false },
    ];

    expect(explorerEntriesToTreePaths(entries)).toEqual(["empty/", "src/index.ts"]);
  });

  it("marks gitignored explorer entries as ignored without overriding SCM status", () => {
    const entries: ExplorerEntry[] = [
      { name: "target", path: "target", isDirectory: true, isSymlink: false, ignored: true },
      { name: "tracked.log", path: "tracked.log", isDirectory: false, isSymlink: false, ignored: true },
      { name: "src", path: "src", isDirectory: true, isSymlink: false, ignored: false },
    ];
    const files: FileEntry[] = [{ path: "tracked.log", status: "modified", renamePath: null }];

    expect(explorerGitStatus(entries, files)).toEqual(
      expect.arrayContaining([
        { path: "tracked.log", status: "modified" },
        { path: "target/", status: "ignored" },
      ])
    );
    expect(explorerGitStatus(entries, files)).toHaveLength(2);
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

  it("needs children only for expanded directories without listed descendants", () => {
    const entries: ExplorerEntry[] = [
      { name: "dist", path: "dist", isDirectory: true, isSymlink: false, ignored: true },
      { name: "index.ts", path: "src/index.ts", isDirectory: false, isSymlink: false, ignored: false },
    ];
    expect(directoryNeedsChildren(entries, "dist")).toBe(true);
    expect(directoryNeedsChildren(entries, "dist/")).toBe(true);
    expect(
      directoryNeedsChildren(
        [...entries, { name: "app.js", path: "dist/app.js", isDirectory: false, isSymlink: false, ignored: true }],
        "dist"
      )
    ).toBe(false);
    expect(directoryNeedsChildren(entries, "src")).toBe(false);
  });
});
