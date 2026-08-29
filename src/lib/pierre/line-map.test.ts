import { describe, it, expect } from "vite-plus/test";
import type { DiffHunk } from "../git-types";
import { mapSelectionToStageLines, normalizeSelectionRange } from "./line-map";

const hunks: DiffHunk[] = [
  {
    header: "@@ -1,2 +1,3 @@",
    oldStart: 1,
    oldLines: 2,
    newStart: 1,
    newLines: 3,
    lines: [
      { content: "keep", lineType: "context", oldLineNumber: 1, newLineNumber: 1 },
      { content: "added-a", lineType: "add", oldLineNumber: null, newLineNumber: 2 },
      { content: "old-b", lineType: "context", oldLineNumber: 2, newLineNumber: 3 },
    ],
  },
  {
    header: "@@ -10,2 +11,2 @@",
    oldStart: 10,
    oldLines: 2,
    newStart: 11,
    newLines: 2,
    lines: [
      { content: "gone", lineType: "remove", oldLineNumber: 10, newLineNumber: null },
      { content: "new", lineType: "add", oldLineNumber: null, newLineNumber: 11 },
    ],
  },
];

describe("mapSelectionToStageLines", () => {
  it("maps a new-side range inside one hunk to 0-based hunk.lines indexes", () => {
    expect(
      mapSelectionToStageLines(hunks, { start: 2, end: 2, side: "additions" })
    ).toEqual([{ hunkIndex: 0, lineStart: 1, lineEnd: 1 }]);
  });

  it("splits a range that spans two hunks", () => {
    expect(
      mapSelectionToStageLines(hunks, {
        start: 2,
        end: 11,
        side: "additions",
        endSide: "additions",
      })
    ).toEqual([
      { hunkIndex: 0, lineStart: 1, lineEnd: 1 },
      { hunkIndex: 1, lineStart: 1, lineEnd: 1 },
    ]);
  });

  it("maps deletions by oldLineNumber", () => {
    expect(
      mapSelectionToStageLines(hunks, { start: 10, end: 10, side: "deletions" })
    ).toEqual([{ hunkIndex: 1, lineStart: 0, lineEnd: 0 }]);
  });

  it("maps a reverse drag after endpoints are normalized", () => {
    const reversed = { start: 11, end: 2, side: "additions" as const, endSide: "additions" as const };
    expect(mapSelectionToStageLines(hunks, reversed)).toEqual([]);
    expect(mapSelectionToStageLines(hunks, normalizeSelectionRange(reversed))).toEqual([
      { hunkIndex: 0, lineStart: 1, lineEnd: 1 },
      { hunkIndex: 1, lineStart: 1, lineEnd: 1 },
    ]);
  });
});

describe("normalizeSelectionRange", () => {
  it("keeps sides when the drag is upward", () => {
    expect(
      normalizeSelectionRange({ start: 11, end: 2, side: "additions", endSide: "additions" })
    ).toEqual({ start: 2, end: 11, side: "additions", endSide: "additions" });
  });
});
