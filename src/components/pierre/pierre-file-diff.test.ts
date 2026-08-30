import { describe, it, expect, vi } from "vite-plus/test";
import type { DiffHunk, RepositoryStatus } from "../../lib/git-types";
import { hunkIdentity } from "../../lib/pierre/hunk-annotations";
import {
  emptyPatchSides,
  enableScmLineSelection,
  historyCacheKey,
  historyFileDiff,
  hunkAnnotations,
  isNonPierreFileType,
  isScmDiffEditable,
  runStageLineCalls,
  scmPatchPresence,
  statusForPath,
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
  it("is true for workingTree and index", () => {
    expect(enableScmLineSelection("workingTree")).toBe(true);
    expect(enableScmLineSelection("index")).toBe(true);
  });

  it("is false for untracked and merge", () => {
    expect(enableScmLineSelection("untracked")).toBe(false);
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
  it("keeps empty blobs when both sides exist", () => {
    expect(emptyPatchSides("src/a.ts", "cache", "old", "", { oldExists: true, newExists: true })).toEqual({
      oldFile: { name: "src/a.ts", contents: "old", cacheKey: "cache" },
      newFile: { name: "src/a.ts", contents: "", cacheKey: "cache" },
    });
    expect(emptyPatchSides("src/a.ts", "cache", "", "", { oldExists: true, newExists: true })).toEqual({
      oldFile: { name: "src/a.ts", contents: "", cacheKey: "cache" },
      newFile: { name: "src/a.ts", contents: "", cacheKey: "cache" },
    });
  });

  it("nulls only a missing side from git status", () => {
    expect(emptyPatchSides("src/a.ts", "cache", "", "new", { oldExists: false, newExists: true })).toEqual({
      oldFile: null,
      newFile: { name: "src/a.ts", contents: "new", cacheKey: "cache" },
    });
    expect(emptyPatchSides("src/a.ts", "cache", "", "", { oldExists: true, newExists: false })).toEqual({
      oldFile: { name: "src/a.ts", contents: "", cacheKey: "cache" },
      newFile: null,
    });
  });
});

describe("scmPatchPresence", () => {
  it("treats untracked and added files as missing the old side", () => {
    expect(scmPatchPresence("untracked", "untracked")).toEqual({ oldExists: false, newExists: true });
    expect(scmPatchPresence("workingTree", "added")).toEqual({ oldExists: false, newExists: true });
    expect(scmPatchPresence("index", "indexAdded")).toEqual({ oldExists: false, newExists: true });
  });

  it("treats deleted files as missing the new side", () => {
    expect(scmPatchPresence("workingTree", "deleted")).toEqual({ oldExists: true, newExists: false });
    expect(scmPatchPresence("index", "indexDeleted")).toEqual({ oldExists: true, newExists: false });
  });

  it("keeps both sides for a tracked empty-file change", () => {
    expect(scmPatchPresence("workingTree", "modified")).toEqual({ oldExists: true, newExists: true });
  });
});

describe("statusForPath", () => {
  it("reads the file status from the matching git group", () => {
    expect(
      statusForPath(
        {
          root: "/repo",
          headBranch: "main",
          headCommit: "abc",
          ahead: 0,
          behind: 0,
          operationState: "none",
          groups: [
            {
              kind: "workingTree",
              label: "Changes",
              files: [{ path: "src/a.ts", status: "deleted", renamePath: null }],
            },
          ],
        },
        "src/a.ts",
        "workingTree"
      )
    ).toBe("deleted");
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
      { side: "additions", lineNumber: 2, metadata: { hunkIndex: 0, identity: hunkIdentity(hunks[0]) } },
      { side: "deletions", lineNumber: 4, metadata: { hunkIndex: 1, identity: hunkIdentity(hunks[1]) } },
    ]);
  });
});

const fakeStatus = (label: string): RepositoryStatus => ({
  root: label,
  headBranch: null,
  headCommit: null,
  ahead: 0,
  behind: 0,
  groups: [],
  operationState: "none",
});

describe("runStageLineCalls", () => {
  it("reidentifies later hunks after an earlier write and publishes each status", async () => {
    const first: DiffHunk = {
      header: "@@ -1,1 +1,2 @@",
      oldStart: 1,
      oldLines: 1,
      newStart: 1,
      newLines: 2,
      lines: [{ content: "a", lineType: "add", oldLineNumber: null, newLineNumber: 1 }],
    };
    const second: DiffHunk = {
      header: "@@ -10,1 +11,2 @@",
      oldStart: 10,
      oldLines: 1,
      newStart: 11,
      newLines: 2,
      lines: [{ content: "b", lineType: "add", oldLineNumber: null, newLineNumber: 11 }],
    };
    const statuses: string[] = [];
    const stageIndexes: number[] = [];
    const onWrote = vi.fn();

    const last = await runStageLineCalls({
      path: "src/a.ts",
      staged: false,
      hunks: [first, second],
      calls: [
        { hunkIndex: 0, lineStart: 0, lineEnd: 0 },
        { hunkIndex: 1, lineStart: 0, lineEnd: 0 },
      ],
      getFileHunks: async () => ({ hunks: [second] }),
      stageLines: async (_path, hunkIndex) => {
        stageIndexes.push(hunkIndex);
        return fakeStatus(`status-${stageIndexes.length}`);
      },
      onStatus: (status) => {
        statuses.push(status.root);
      },
      onWrote,
    });

    expect(stageIndexes).toEqual([0, 0]);
    expect(statuses).toEqual(["status-1", "status-2"]);
    expect(last?.root).toBe("status-2");
    expect(onWrote).toHaveBeenCalledTimes(1);
  });

  it("still publishes the successful write when a later call fails", async () => {
    const first: DiffHunk = {
      header: "@@ -1,1 +1,2 @@",
      oldStart: 1,
      oldLines: 1,
      newStart: 1,
      newLines: 2,
      lines: [{ content: "a", lineType: "add", oldLineNumber: null, newLineNumber: 1 }],
    };
    const second: DiffHunk = {
      header: "@@ -10,1 +11,2 @@",
      oldStart: 10,
      oldLines: 1,
      newStart: 11,
      newLines: 2,
      lines: [{ content: "b", lineType: "add", oldLineNumber: null, newLineNumber: 11 }],
    };
    const statuses: string[] = [];
    const onWrote = vi.fn();

    await expect(
      runStageLineCalls({
        path: "src/a.ts",
        staged: false,
        hunks: [first, second],
        calls: [
          { hunkIndex: 0, lineStart: 0, lineEnd: 0 },
          { hunkIndex: 1, lineStart: 0, lineEnd: 0 },
        ],
        getFileHunks: async () => ({ hunks: [second] }),
        stageLines: async (_path, hunkIndex) => {
          if (hunkIndex === 0 && statuses.length === 1) throw new Error("later failed");
          return fakeStatus("ok");
        },
        onStatus: (status) => {
          statuses.push(status.root);
        },
        onWrote,
      })
    ).rejects.toThrow("later failed");

    expect(statuses).toEqual(["ok"]);
    expect(onWrote).toHaveBeenCalledTimes(1);
  });
});

describe("historyCacheKey", () => {
  it("includes the commit id so the same path does not reuse tokens", () => {
    expect(historyCacheKey("abc123", "src/a.ts")).toBe("abc123:src/a.ts");
    expect(historyCacheKey("abc123", "src/a.ts")).not.toBe(historyCacheKey("def456", "src/a.ts"));
  });
});

describe("historyFileDiff", () => {
  it("keeps a context hunk when both sides are identical", () => {
    const diff = historyFileDiff("src/a.ts", "hello\n", "hello\n", "abc:src/a.ts");
    expect(diff.hunks).toHaveLength(1);
    expect(diff.hunks[0].hunkContent).toEqual([
      { type: "context", lines: 1, additionLineIndex: 0, deletionLineIndex: 0 },
    ]);
    expect(diff.additionLines).toEqual(["hello\n"]);
    expect(diff.deletionLines).toEqual(["hello\n"]);
    expect(diff.splitLineCount).toBe(1);
    expect(diff.cacheKey).toBe("abc:src/a.ts");
  });

  it("parses a change when the sides differ", () => {
    const diff = historyFileDiff("src/a.ts", "hello\n", "hello\nworld\n", "def:src/a.ts");
    expect(diff.hunks[0].hunkContent.some((block) => block.type === "change")).toBe(true);
    expect(diff.cacheKey).toBe("def:src/a.ts");
  });
});
