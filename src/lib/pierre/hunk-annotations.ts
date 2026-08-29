import type { DiffHunk } from "../git-types";

export type HunkActionSide = "additions" | "deletions";

export const hunkActionAnchor = (
  hunk: DiffHunk
): { side: HunkActionSide; lineNumber: number } | null => {
  const addition = hunk.lines.find((line) => line.lineType === "add" && line.newLineNumber !== null);
  if (addition?.newLineNumber != null) {
    return { side: "additions", lineNumber: addition.newLineNumber };
  }
  const deletion = hunk.lines.find((line) => line.lineType === "remove" && line.oldLineNumber !== null);
  if (deletion?.oldLineNumber != null) {
    return { side: "deletions", lineNumber: deletion.oldLineNumber };
  }
  return null;
};
