import { describe, it, expect } from "vite-plus/test";
import { enqueueMergeResolve, shouldMountMergePane } from "./pierre-unresolved";

describe("shouldMountMergePane", () => {
  it("mounts only when the stored diff belongs to the selected merge file", () => {
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "merge" }, { path: "src/a.ts" })).toBe(true);
    expect(shouldMountMergePane({ path: "src/b.ts", groupKind: "merge" }, { path: "src/a.ts" })).toBe(false);
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "workingTree" }, { path: "src/a.ts" })).toBe(false);
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "merge" }, null)).toBe(false);
  });
});

describe("enqueueMergeResolve", () => {
  it("serializes write-then-stage work for the same path", async () => {
    const order: number[] = [];
    let releaseFirst!: () => void;
    const gate = new Promise<void>((resolve) => {
      releaseFirst = resolve;
    });
    const first = enqueueMergeResolve("src/a.ts", async () => {
      await gate;
      order.push(1);
    });
    const second = enqueueMergeResolve("src/a.ts", async () => {
      order.push(2);
    });

    expect(order).toEqual([]);
    releaseFirst();
    await Promise.all([first, second]);
    expect(order).toEqual([1, 2]);
  });
});
