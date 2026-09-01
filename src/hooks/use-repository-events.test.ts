import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import type { PathsChanged, StatusEntry, StatusPatch, StatusSnapshot } from "../lib/git-types";
import { applyIncomingPatch, pathsChangedIntersects } from "./use-repository-events";
import { resetStatusStore, statusStore } from "../stores/status-store";
import { repositoryStore } from "../stores/repository-store";

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
  repositoryStore.setState({
    status: {
      root: "/repo",
      headBranch: "main",
      headCommit: null,
      ahead: 0,
      behind: 0,
      groups: [],
      operationState: "none",
    },
    selectedFile: { path: "gone.ts", staged: false, groupKind: "workingTree" },
    selectedLoadId: 1,
    diff: { path: "gone.ts", original: "", modified: "x", originalLanguage: null, fileType: "text" },
    diffLoadId: 1,
    blame: null,
    cursorLine: null,
  });
});

describe("applyIncomingPatch", () => {
  it("applies a patch and projects groups onto repository status", async () => {
    const result = await applyIncomingPatch(
      patch({
        upserts: [entry("a.ts")],
        metadata: {
          root: "/repo",
          headBranch: "main",
          headCommit: "abc",
          ahead: 1,
          behind: 0,
          operationState: "none",
        },
      }),
      async () => {
        throw new Error("should not recover");
      }
    );

    expect(result).toBe("applied");
    expect(statusStore.getState().groups[0]?.files[0]?.path).toBe("a.ts");
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("a.ts");
    expect(repositoryStore.getState().status?.ahead).toBe(1);
    expect(repositoryStore.getState().status?.headCommit).toBe("abc");
  });

  it("recovers from a revision gap via snapshot", async () => {
    await applyIncomingPatch(patch({ upserts: [entry("a.ts")] }), async () => {
      throw new Error("should not recover");
    });

    const snapshot: StatusSnapshot = {
      generation: 1,
      revision: 5,
      phase: "settled",
      metadata: {
        root: "/repo",
        headBranch: "main",
        headCommit: "def",
        ahead: 0,
        behind: 2,
        operationState: "none",
      },
      entries: [entry("recovered.ts", "index")],
    };
    const recover = vi.fn(async () => snapshot);

    const result = await applyIncomingPatch(patch({ baseRevision: 4, revision: 5, upserts: [entry("skip.ts")] }), recover);

    expect(result).toBe("gap");
    expect(recover).toHaveBeenCalledOnce();
    expect(statusStore.getState().revision).toBe(5);
    expect(statusStore.getState().groups.map((group) => group.kind)).toEqual(["index"]);
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("recovered.ts");
    expect(repositoryStore.getState().status?.behind).toBe(2);
  });

  it("clears the selected file when it leaves the status map", async () => {
    await applyIncomingPatch(patch({ upserts: [entry("kept.ts")] }), async () => {
      throw new Error("should not recover");
    });

    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().diff).toBeNull();
  });
});

describe("pathsChangedIntersects", () => {
  const event = (overrides: Partial<PathsChanged>): PathsChanged => ({
    paths: [],
    kind: "content",
    scope: "exact",
    generation: 1,
    storm: false,
    ...overrides,
  });

  it("matches an exact path and repository scope", () => {
    expect(pathsChangedIntersects(event({ paths: ["src/a.ts"], scope: "exact" }), "src/a.ts")).toBe(true);
    expect(pathsChangedIntersects(event({ paths: ["src/a.ts"], scope: "exact" }), "src/b.ts")).toBe(false);
    expect(pathsChangedIntersects(event({ paths: [], scope: "repository" }), "anything.ts")).toBe(true);
  });

  it("matches a subtree path", () => {
    expect(pathsChangedIntersects(event({ paths: ["src"], scope: "subtree" }), "src/a.ts")).toBe(true);
    expect(pathsChangedIntersects(event({ paths: ["src"], scope: "subtree" }), "lib/a.ts")).toBe(false);
  });
});
