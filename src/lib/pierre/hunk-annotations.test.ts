import { describe, it, expect } from "vite-plus/test";
import type { DiffHunk } from "../git-types";
import { hunkActionAnchor, hunkIdentity, reidentifyHunk } from "./hunk-annotations";

describe("hunkActionAnchor", () => {
  it("anchors on the first addition line when present", () => {
    const hunk: DiffHunk = {
      header: "",
      oldStart: 1,
      oldLines: 1,
      newStart: 1,
      newLines: 2,
      lines: [
        { content: "a", lineType: "context", oldLineNumber: 1, newLineNumber: 1 },
        { content: "b", lineType: "add", oldLineNumber: null, newLineNumber: 2 },
      ],
    };
    expect(hunkActionAnchor(hunk)).toEqual({ side: "additions", lineNumber: 2 });
  });

  it("anchors on the first deletion when the hunk has no additions", () => {
    const hunk: DiffHunk = {
      header: "",
      oldStart: 4,
      oldLines: 1,
      newStart: 4,
      newLines: 0,
      lines: [{ content: "x", lineType: "remove", oldLineNumber: 4, newLineNumber: null }],
    };
    expect(hunkActionAnchor(hunk)).toEqual({ side: "deletions", lineNumber: 4 });
  });

  it("returns null when the hunk has no additions or deletions", () => {
    const hunk: DiffHunk = {
      header: "",
      oldStart: 1,
      oldLines: 1,
      newStart: 1,
      newLines: 1,
      lines: [{ content: "a", lineType: "context", oldLineNumber: 1, newLineNumber: 1 }],
    };
    expect(hunkActionAnchor(hunk)).toBeNull();
  });
});

const changedHunk = (header: string, oldStart: number, newStart: number, added: string): DiffHunk => ({
  header,
  oldStart,
  oldLines: 1,
  newStart,
  newLines: 2,
  lines: [
    { content: "keep", lineType: "context", oldLineNumber: oldStart, newLineNumber: newStart },
    { content: added, lineType: "add", oldLineNumber: null, newLineNumber: newStart + 1 },
  ],
});

describe("reidentifyHunk", () => {
  it("finds the same changed lines after a preceding hunk is inserted", () => {
    const target = changedHunk("@@ -10,1 +10,2 @@", 10, 10, "later");
    const refreshed = [changedHunk("@@ -1,1 +1,2 @@", 1, 1, "earlier"), target];
    expect(reidentifyHunk(refreshed, hunkIdentity(target))).toBe(1);
  });

  it("finds the same changed lines after a preceding hunk is removed", () => {
    const first = changedHunk("@@ -1,1 +1,2 @@", 1, 1, "earlier");
    const target = changedHunk("@@ -10,1 +10,2 @@", 10, 10, "later");
    expect(reidentifyHunk([target], hunkIdentity(target))).toBe(0);
    expect(reidentifyHunk([target], hunkIdentity(first))).toBeNull();
  });
});
