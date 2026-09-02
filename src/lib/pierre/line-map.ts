export type PierreLineRange = {
  start: number;
  end: number;
  side: "additions" | "deletions";
  endSide?: "additions" | "deletions";
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
