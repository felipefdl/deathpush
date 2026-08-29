import { describe, it, expect } from "vite-plus/test";
import type { DiffHunk } from "../git-types";
import { hunkActionAnchor } from "./hunk-annotations";

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
