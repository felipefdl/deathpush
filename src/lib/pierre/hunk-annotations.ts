import type { DiffHunk } from "../git-types";

export type HunkActionSide = "additions" | "deletions";

export type HunkIdentity = {
  header: string;
  oldStart: number;
  newStart: number;
  fingerprint: string;
};

export const hunkFingerprint = (hunk: DiffHunk): string =>
  hunk.lines
    .filter((line) => line.lineType === "add" || line.lineType === "remove")
    .map((line) => `${line.lineType}\0${line.content}`)
    .join("\n");

export const hunkIdentity = (hunk: DiffHunk): HunkIdentity => ({
  header: hunk.header,
  oldStart: hunk.oldStart,
  newStart: hunk.newStart,
  fingerprint: hunkFingerprint(hunk),
});

export const reidentifyHunk = (hunks: DiffHunk[], identity: HunkIdentity): number | null => {
  const fingerprintHits = hunks.flatMap((hunk, index) =>
    hunkFingerprint(hunk) === identity.fingerprint ? [index] : []
  );
  if (fingerprintHits.length === 1) return fingerprintHits[0];
  if (fingerprintHits.length > 1) {
    const byStart = fingerprintHits.filter(
      (index) => hunks[index].oldStart === identity.oldStart && hunks[index].newStart === identity.newStart
    );
    if (byStart.length === 1) return byStart[0];
    const byHeader = fingerprintHits.filter((index) => hunks[index].header === identity.header);
    if (byHeader.length === 1) return byHeader[0];
  }
  const headerHits = hunks.flatMap((hunk, index) =>
    hunk.header === identity.header && hunk.header !== "" ? [index] : []
  );
  if (headerHits.length === 1) return headerHits[0];
  const startHits = hunks.flatMap((hunk, index) =>
    hunk.oldStart === identity.oldStart && hunk.newStart === identity.newStart ? [index] : []
  );
  if (startHits.length === 1) return startHits[0];
  return null;
};

export const hunkActionAnchor = (hunk: DiffHunk): { side: HunkActionSide; lineNumber: number } | null => {
  const addition = hunk.lines.find((line) => line.lineType === "add" && line.newLineNumber !== null);
  if (addition !== undefined && addition.newLineNumber !== null) {
    return { side: "additions", lineNumber: addition.newLineNumber };
  }
  const deletion = hunk.lines.find((line) => line.lineType === "remove" && line.oldLineNumber !== null);
  if (deletion !== undefined && deletion.oldLineNumber !== null) {
    return { side: "deletions", lineNumber: deletion.oldLineNumber };
  }
  return null;
};
