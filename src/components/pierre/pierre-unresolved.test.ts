import { describe, it, expect } from "vite-plus/test";
import { enqueueMergeResolve, shouldMountMergePane } from "./pierre-unresolved";

describe("shouldMountMergePane", () => {
  it("mounts only when the stored diff belongs to the selected merge load", () => {
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "merge" }, 1, { path: "src/a.ts" }, 1)).toBe(true);
    expect(shouldMountMergePane({ path: "src/b.ts", groupKind: "merge" }, 1, { path: "src/a.ts" }, 1)).toBe(false);
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "workingTree" }, 1, { path: "src/a.ts" }, 1)).toBe(
      false
    );
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "merge" }, 1, null, 1)).toBe(false);
    expect(shouldMountMergePane({ path: "src/a.ts", groupKind: "merge" }, 2, { path: "src/a.ts" }, 1)).toBe(false);
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
