import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import { applyStatusPatch, resetStatusStore, statusStore } from "../stores/status-store";
import { beginRepositorySession, enqueueStatusPatch, flushPendingPatches } from "./use-repository-events";
import { useRepository } from "./use-repository";
import type { StatusEntry, StatusPatch } from "../lib/git-types";

const identity = {
  root: "/test/project",
  headBranch: "main",
};

const { openRepositoryMock, getStatusMock } = vi.hoisted(() => ({
  openRepositoryMock: vi.fn(),
  getStatusMock: vi.fn(),
}));

vi.mock("../lib/tauri-commands", () => ({
  openRepository: openRepositoryMock,
  getStatus: getStatusMock,
}));

describe("openRepo", () => {
  beforeEach(() => {
    repositoryStore.setState({ status: null, error: null, operations: new Set() });
    resetStatusStore();
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
    openRepositoryMock.mockReset();
    getStatusMock.mockReset();
  });

  it("shows repository identity without applying a full status snapshot", async () => {
    let resolveOpen!: (value: typeof identity) => void;
    let resolveStatus!: () => void;
    const openGate = new Promise<typeof identity>((resolve) => {
      resolveOpen = resolve;
    });
    const statusGate = new Promise<void>((resolve) => {
      resolveStatus = resolve;
    });
    openRepositoryMock.mockImplementation(() => openGate);
    getStatusMock.mockImplementation(() => statusGate);

    const pending = useRepository().openRepo("/test/project");
    await Promise.resolve();

    expect(repositoryStore.getState().operations.has("open-repo")).toBe(true);
    expect(repositoryStore.getState().status).toBeNull();

    resolveOpen(identity);
    await Promise.resolve();
    await Promise.resolve();

    expect(repositoryStore.getState().status).toEqual({
      root: "/test/project",
      headBranch: "main",
      headCommit: null,
      ahead: 0,
      behind: 0,
      groups: [],
      operationState: "none",
    });
    expect(getStatusMock).toHaveBeenCalledOnce();
    expect(repositoryStore.getState().operations.has("open-repo")).toBe(false);
    expect(statusStore.getState().groups).toEqual([]);

    resolveStatus();
    await pending;

    expect(repositoryStore.getState().status?.groups).toEqual([]);
    expect(statusStore.getState().groups).toEqual([]);
  });

  it("does not apply old-repository patches that arrive while opening a new repository", async () => {
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

    beginRepositorySession();
    repositoryStore.getState().setIdentity({ root: "/repo-a", headBranch: "main" });
    applyStatusPatch(patch({ upserts: [entry("from-a.ts")] }));
    repositoryStore.getState().syncStatusGroups();

    let resolveOpen!: (value: { root: string; headBranch: string }) => void;
    const openGate = new Promise<{ root: string; headBranch: string }>((resolve) => {
      resolveOpen = resolve;
    });
    openRepositoryMock.mockImplementation(() => openGate);
    getStatusMock.mockResolvedValue(undefined);

    const pending = useRepository().openRepo("/repo-b");
    await Promise.resolve();
    await Promise.resolve();

    enqueueStatusPatch(patch({ upserts: [entry("late-a.ts")] }));

    resolveOpen({ root: "/repo-b", headBranch: "main" });
    await pending;
    await flushPendingPatches();

    const files = statusStore
      .getState()
      .groups.flatMap((group) => group.files)
      .map((file) => file.path);
    expect(files).not.toContain("late-a.ts");
    expect(files).not.toContain("from-a.ts");
    expect(repositoryStore.getState().status?.root).toBe("/repo-b");
  });
});
