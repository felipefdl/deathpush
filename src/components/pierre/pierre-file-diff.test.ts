import { describe, it, expect } from "vite-plus/test";
import type { DiffHunk } from "../../lib/git-types";
import {
  emptyPatchSides,
  enableScmLineSelection,
  hunkAnnotations,
  isNonPierreFileType,
  isScmDiffEditable,
} from "./pierre-file-diff";

describe("isScmDiffEditable", () => {
  it("is true for a working-tree side that is not index or merge", () => {
    expect(isScmDiffEditable("workingTree", true)).toBe(true);
    expect(isScmDiffEditable("untracked", true)).toBe(true);
  });

  it("is false for index, merge, or a missing working-tree side", () => {
    expect(isScmDiffEditable("index", true)).toBe(false);
    expect(isScmDiffEditable("merge", true)).toBe(false);
    expect(isScmDiffEditable("workingTree", false)).toBe(false);
  });
});

describe("enableScmLineSelection", () => {
  it("is true for workingTree, untracked, and index", () => {
    expect(enableScmLineSelection("workingTree")).toBe(true);
    expect(enableScmLineSelection("untracked")).toBe(true);
    expect(enableScmLineSelection("index")).toBe(true);
  });

  it("is false for merge", () => {
    expect(enableScmLineSelection("merge")).toBe(false);
  });
});

describe("isNonPierreFileType", () => {
  it("skips image, binary, and large", () => {
    expect(isNonPierreFileType("image")).toBe(true);
    expect(isNonPierreFileType("binary")).toBe(true);
    expect(isNonPierreFileType("large")).toBe(true);
    expect(isNonPierreFileType("text")).toBe(false);
  });
});

describe("emptyPatchSides", () => {
  it("nulls the missing side for add and delete", () => {
    expect(emptyPatchSides("src/a.ts", "cache", "", "new")).toEqual({
      oldFile: null,
      newFile: { name: "src/a.ts", contents: "new", cacheKey: "cache" },
    });
    expect(emptyPatchSides("src/a.ts", "cache", "old", "")).toEqual({
      oldFile: { name: "src/a.ts", contents: "old", cacheKey: "cache" },
      newFile: null,
    });
  });

  it("keeps an empty new side when both blobs are empty", () => {
    expect(emptyPatchSides("src/a.ts", "cache", "", "")).toEqual({
      oldFile: null,
      newFile: { name: "src/a.ts", contents: "", cacheKey: "cache" },
    });
  });
});

describe("hunkAnnotations", () => {
  it("anchors each changed hunk and skips context-only hunks", () => {
    const hunks: DiffHunk[] = [
      {
        header: "",
        oldStart: 1,
        oldLines: 1,
        newStart: 1,
        newLines: 2,
        lines: [
          { content: "a", lineType: "context", oldLineNumber: 1, newLineNumber: 1 },
          { content: "b", lineType: "add", oldLineNumber: null, newLineNumber: 2 },
        ],
      },
      {
        header: "",
        oldStart: 4,
        oldLines: 1,
        newStart: 4,
        newLines: 0,
        lines: [{ content: "x", lineType: "remove", oldLineNumber: 4, newLineNumber: null }],
      },
      {
        header: "",
        oldStart: 8,
        oldLines: 1,
        newStart: 8,
        newLines: 1,
        lines: [{ content: "a", lineType: "context", oldLineNumber: 8, newLineNumber: 8 }],
      },
    ];

    expect(hunkAnnotations(hunks)).toEqual([
      { side: "additions", lineNumber: 2, metadata: { hunkIndex: 0 } },
      { side: "deletions", lineNumber: 4, metadata: { hunkIndex: 1 } },
    ]);
  });
});
