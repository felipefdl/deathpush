import { describe, it, expect } from "vitest";
import { buildFlatFileList } from "./flat-file-list";
import type { ResourceGroup } from "./git-types";

const makeGroup = (kind: ResourceGroup["kind"], label: string, paths: string[]): ResourceGroup => ({
  kind,
  label,
  files: paths.map((path) => ({ path, status: "modified", renamePath: null })),
});

describe("buildFlatFileList", () => {
  it("returns empty array for empty groups", () => {
    expect(buildFlatFileList([], "")).toEqual([]);
  });

  it("returns all files from a single group", () => {
    const groups = [makeGroup("index", "Staged", ["src/a.ts", "src/b.ts", "src/c.ts"])];
    const result = buildFlatFileList(groups, "");
    expect(result).toEqual([
      { path: "src/a.ts", groupKind: "index" },
      { path: "src/b.ts", groupKind: "index" },
      { path: "src/c.ts", groupKind: "index" },
    ]);
  });

  it("returns files from multiple groups preserving order", () => {
    const groups = [
      makeGroup("index", "Staged", ["src/a.ts"]),
      makeGroup("workingTree", "Changes", ["src/b.ts"]),
      makeGroup("untracked", "Untracked", ["src/c.ts"]),
    ];
    const result = buildFlatFileList(groups, "");
    expect(result).toEqual([
      { path: "src/a.ts", groupKind: "index" },
      { path: "src/b.ts", groupKind: "workingTree" },
      { path: "src/c.ts", groupKind: "untracked" },
    ]);
  });

  it("filters case-insensitively", () => {
    const groups = [makeGroup("index", "Staged", ["src/App.tsx", "src/utils.ts"])];
    const result = buildFlatFileList(groups, "app");
    expect(result).toEqual([{ path: "src/App.tsx", groupKind: "index" }]);
  });

  it("returns empty array when filter matches nothing", () => {
    const groups = [makeGroup("index", "Staged", ["src/a.ts", "src/b.ts"])];
    expect(buildFlatFileList(groups, "zzz")).toEqual([]);
  });

  it("returns all files when filter is empty string", () => {
    const groups = [makeGroup("workingTree", "Changes", ["x.ts", "y.ts"])];
    const result = buildFlatFileList(groups, "");
    expect(result).toHaveLength(2);
  });

  it("handles a group with empty files array", () => {
    const groups = [makeGroup("index", "Staged", [])];
    expect(buildFlatFileList(groups, "")).toEqual([]);
  });

  it("preserves groupKind on filtered results", () => {
    const groups = [makeGroup("merge", "Merge", ["conflict.ts"])];
    const result = buildFlatFileList(groups, "conflict");
    expect(result).toEqual([{ path: "conflict.ts", groupKind: "merge" }]);
  });

  it("matches substring in middle of path", () => {
    const groups = [makeGroup("index", "Staged", ["src/components/Button.tsx"])];
    const result = buildFlatFileList(groups, "components");
    expect(result).toEqual([{ path: "src/components/Button.tsx", groupKind: "index" }]);
  });

  it("returns files only from groups that match the filter", () => {
    const groups = [
      makeGroup("index", "Staged", ["src/foo.ts"]),
      makeGroup("workingTree", "Changes", ["lib/bar.ts"]),
    ];
    const result = buildFlatFileList(groups, "bar");
    expect(result).toEqual([{ path: "lib/bar.ts", groupKind: "workingTree" }]);
  });
});
