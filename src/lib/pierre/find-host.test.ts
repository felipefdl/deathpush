import { describe, it, expect } from "vite-plus/test";
import { scanPierreFind } from "./find-host";

const line = (text: string): HTMLElement => {
  const element = document.createElement("span");
  element.setAttribute("data-line", "1");
  element.textContent = text;
  return element;
};

const root = (elements: HTMLElement[]): { querySelectorAll: () => HTMLElement[] } => ({
  querySelectorAll: () => elements,
});

describe("scanPierreFind", () => {
  it("returns a range for each case-insensitive match", () => {
    const ranges = scanPierreFind(root([line("alpha Beta alpha")]), "alpha");
    expect(ranges).toHaveLength(2);
    expect(ranges[0].toString()).toBe("alpha");
    expect(ranges[1].toString()).toBe("alpha");
  });

  it("returns no ranges when the needle is missing", () => {
    expect(scanPierreFind(root([line("alpha beta")]), "gamma")).toEqual([]);
  });

  it("returns no ranges for a blank query", () => {
    expect(scanPierreFind(root([line("alpha")]), "   ")).toEqual([]);
  });
});
