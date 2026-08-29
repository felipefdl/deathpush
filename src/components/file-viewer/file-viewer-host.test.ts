import { describe, it, expect } from "vite-plus/test";
import { isPierreHostReady } from "./file-viewer";

describe("isPierreHostReady", () => {
  it("is false until selected path, loaded content, and session share one path", () => {
    expect(isPierreHostReady("src/b.ts", { path: "src/a.ts" }, { path: "src/a.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, { path: "src/a.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", null, { path: "src/b.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, null)).toBe(false);
  });

  it("is true only when the host can use the same loaded file", () => {
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, { path: "src/b.ts" })).toBe(true);
  });
});
