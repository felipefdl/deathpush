import { describe, it, expect } from "vite-plus/test";
import { normalizeSelectionRange } from "./line-map";

describe("normalizeSelectionRange", () => {
  it("keeps sides when the drag is upward", () => {
    expect(normalizeSelectionRange({ start: 11, end: 2, side: "additions", endSide: "additions" })).toEqual({
      start: 2,
      end: 11,
      side: "additions",
      endSide: "additions",
    });
  });
});
