import type { DiffHunk, DiffLine } from "../git-types";

export type PierreLineRange = {
  start: number;
  end: number;
  side: "additions" | "deletions";
  endSide?: "additions" | "deletions";
};

export type StageLinesCall = {
  hunkIndex: number;
  lineStart: number;
  lineEnd: number;
};

export const normalizeSelectionRange = (range: {
  start: number;
  end: number;
  side?: "additions" | "deletions";
  endSide?: "additions" | "deletions";
}): PierreLineRange => {
  if (range.start <= range.end) {
    return {
      start: range.start,
      end: range.end,
      side: range.side ?? "additions",
      endSide: range.endSide,
    };
  }
  return {
    start: range.end,
    end: range.start,
    side: range.side ?? "additions",
    endSide: range.endSide,
  };
};

const inRange = (lineNumber: number | null, start: number, end: number): boolean =>
  lineNumber !== null && lineNumber >= start && lineNumber <= end;

const wantsSide = (range: PierreLineRange, side: "additions" | "deletions"): boolean =>
  range.side === side || range.endSide === side;

const matchesSelection = (line: DiffLine, range: PierreLineRange): boolean => {
  if (line.lineType === "add") {
    return wantsSide(range, "additions") && inRange(line.newLineNumber, range.start, range.end);
  }
  if (line.lineType === "remove") {
    return wantsSide(range, "deletions") && inRange(line.oldLineNumber, range.start, range.end);
  }
  if (line.lineType === "context" && range.endSide === undefined) {
    const lineNumber = range.side === "additions" ? line.newLineNumber : line.oldLineNumber;
    return inRange(lineNumber, range.start, range.end);
  }
  return false;
};

export const mapSelectionToStageLines = (hunks: DiffHunk[], range: PierreLineRange): StageLinesCall[] => {
  const calls: StageLinesCall[] = [];
  for (const [hunkIndex, hunk] of hunks.entries()) {
    const indexes: number[] = [];
    for (const [index, line] of hunk.lines.entries()) {
      if (matchesSelection(line, range)) {
        indexes.push(index);
      }
    }
    if (indexes.length === 0) {
      continue;
    }
    calls.push({
      hunkIndex,
      lineStart: Math.min(...indexes),
      lineEnd: Math.max(...indexes),
    });
  }
  return calls;
};
