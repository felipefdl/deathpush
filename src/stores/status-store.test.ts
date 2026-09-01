import { beforeEach, describe, expect, it } from "vite-plus/test";
import type { StatusEntry, StatusPatch } from "../lib/git-types";
import { applyStatusPatch, replaceFromSnapshot, resetStatusStore, statusStore } from "./status-store";

const entry = (path: string, group: StatusEntry["group"] = "workingTree"): StatusEntry => ({
  group,
  path,
  status: "modified",
  renamePath: null,
});

const patch = (overrides: Partial<StatusPatch> = {}): StatusPatch => ({
  generation: 1,
  baseRevision: 0,
  revision: 1,
  upserts: [],
  removals: [],
  phase: "settled",
  ...overrides,
});

beforeEach(() => {
  resetStatusStore();
});

describe("applyStatusPatch", () => {
  it("upserts entries into the map and rebuilds groups", () => {
    const result = applyStatusPatch(
      patch({
        upserts: [entry("a.ts"), entry("b.ts", "index")],
      })
    );

    expect(result).toBe("applied");
    const state = statusStore.getState();
    expect(state.revision).toBe(1);
    expect(state.generation).toBe(1);
    expect(state.groups.map((group) => group.kind)).toEqual(["index", "workingTree"]);
    expect(state.groups.find((group) => group.kind === "index")?.files).toEqual([
      { path: "b.ts", status: "modified", renamePath: null },
    ]);
    expect(state.groups.find((group) => group.kind === "workingTree")?.files).toEqual([
      { path: "a.ts", status: "modified", renamePath: null },
    ]);
  });

  it("removes entries by group and path", () => {
    applyStatusPatch(patch({ upserts: [entry("a.ts"), entry("b.ts")] }));
    const result = applyStatusPatch(
      patch({
        baseRevision: 1,
        revision: 2,
        removals: [{ group: "workingTree", path: "a.ts" }],
      })
    );

    expect(result).toBe("applied");
    const files = statusStore.getState().groups.find((group) => group.kind === "workingTree")?.files ?? [];
    expect(files.map((file) => file.path)).toEqual(["b.ts"]);
  });

  it("discards patches from an older generation", () => {
    applyStatusPatch(patch({ generation: 2, upserts: [entry("kept.ts")] }));
    const result = applyStatusPatch(
      patch({
        generation: 1,
        baseRevision: 1,
        revision: 2,
        upserts: [entry("stale.ts")],
      })
    );

    expect(result).toBe("discarded");
    const files = statusStore.getState().groups.flatMap((group) => group.files);
    expect(files.map((file) => file.path)).toEqual(["kept.ts"]);
    expect(statusStore.getState().revision).toBe(1);
  });

  it("reports a revision gap without applying the patch", () => {
    applyStatusPatch(patch({ upserts: [entry("a.ts")] }));
    const result = applyStatusPatch(
      patch({
        baseRevision: 4,
        revision: 5,
        upserts: [entry("skipped.ts")],
        removals: [{ group: "workingTree", path: "a.ts" }],
      })
    );

    expect(result).toBe("gap");
    const files = statusStore.getState().groups.flatMap((group) => group.files);
    expect(files.map((file) => file.path)).toEqual(["a.ts"]);
    expect(statusStore.getState().revision).toBe(1);
  });

  it("keeps unrelated group file arrays when updating one group", () => {
    applyStatusPatch(patch({ upserts: [entry("a.ts"), entry("b.ts", "index")] }));
    const indexFiles = statusStore.getState().groups.find((group) => group.kind === "index")?.files;

    applyStatusPatch(
      patch({
        baseRevision: 1,
        revision: 2,
        upserts: [entry("c.ts")],
      })
    );

    expect(statusStore.getState().groups.find((group) => group.kind === "index")?.files).toBe(indexFiles);
    const workingTreeFiles = statusStore.getState().groups.find((group) => group.kind === "workingTree")?.files;
    expect(workingTreeFiles?.map((file) => file.path)).toEqual(["a.ts", "c.ts"]);
  });
});

describe("replaceFromSnapshot", () => {
  it("replaces the map from a recovery snapshot", () => {
    applyStatusPatch(patch({ upserts: [entry("old.ts")] }));
    replaceFromSnapshot({
      generation: 3,
      revision: 9,
      phase: "scanning",
      metadata: {
        root: "/repo",
        headBranch: "main",
        headCommit: "abc",
        ahead: 1,
        behind: 0,
        operationState: "none",
      },
      entries: [entry("new.ts", "index")],
    });

    const state = statusStore.getState();
    expect(state.generation).toBe(3);
    expect(state.revision).toBe(9);
    expect(state.phase).toBe("scanning");
    expect(state.groups.map((group) => group.kind)).toEqual(["index"]);
    expect(state.groups[0]?.files[0]?.path).toBe("new.ts");
  });
});
