import { describe, it, expect, vi } from "vite-plus/test";
import type { DiffHunk } from "../../lib/git-types";
import {
  emptyPatchSides,
  historyCacheKey,
  historyFileDiff,
  hunkAnnotations,
  loadScmDiffSources,
  isNonPierreFileType,
} from "./pierre-file-diff";

const { sendIntentMock } = vi.hoisted(() => ({ sendIntentMock: vi.fn() }));

vi.mock("../../lib/session-client", () => ({
  sendIntent: sendIntentMock,
  sendDestructiveIntent: vi.fn(),
  applySessionSnapshot: vi.fn(),
}));

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
    const withIds = hunks.map((hunk, index) => ({ ...hunk, id: `h${index}` }));

    expect(hunkAnnotations(withIds)).toEqual([
      { side: "additions", lineNumber: 2, metadata: { hunkId: "h0" } },
      { side: "deletions", lineNumber: 4, metadata: { hunkId: "h1" } },
    ]);
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

describe("loadScmDiffSources", () => {
  it("sends an openScmDiff intent", async () => {
    const payload = {
      path: "README.md",
      original: "",
      modified: "contents",
      language: null,
      fileType: "text",
      hunks: [],
      presence: { oldExists: true, newExists: true },
      editable: true,
      enableLineSelection: true,
      staged: false,
      contentHash: "hash-contents",
    };
    sendIntentMock.mockResolvedValue({ kind: "diff", payload });
    await expect(
      loadScmDiffSources({
        path: "README.md",
        staged: false,
        groupKind: "workingTree",
        loadId: 1,
        consumeCache: true,
      })
    ).resolves.toEqual(payload);
    expect(sendIntentMock).toHaveBeenCalledWith({
      type: "openScmDiff",
      path: "README.md",
      staged: false,
      groupKind: "workingTree",
    });
  });
});
