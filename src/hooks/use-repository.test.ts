import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import { applyStatusPatch, resetStatusStore, statusStore } from "../stores/status-store";
import { beginRepositorySession, enqueueStatusPatch, flushPendingPatches } from "./use-repository-events";
import { recoverFromSnapshot, useRepository } from "./use-repository";
import type { StatusEntry, StatusPatch, StatusSnapshot } from "../lib/git-types";

const identity = {
  root: "/test/project",
  headBranch: "main",
};

const { openRepositoryMock, getStatusMock, getStatusSnapshotMock, refreshStatusMock } = vi.hoisted(() => ({
  openRepositoryMock: vi.fn(),
  getStatusMock: vi.fn(),
  getStatusSnapshotMock: vi.fn(),
  refreshStatusMock: vi.fn(),
}));

vi.mock("../lib/tauri-commands", () => ({
  openRepository: openRepositoryMock,
  getStatus: getStatusMock,
  getStatusSnapshot: getStatusSnapshotMock,
  refreshStatus: refreshStatusMock,
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
    getStatusSnapshotMock.mockReset();
    refreshStatusMock.mockReset();
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

  it("populates groups from a snapshot after new-repo patches arrive during open await", async () => {
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
    const snapshot: StatusSnapshot = {
      generation: 2,
      revision: 4,
      phase: "settled",
      entries: [entry("from-b.ts")],
      metadata: {
        root: "/repo-b",
        headBranch: "main",
        headCommit: "abc",
        ahead: 0,
        behind: 0,
        operationState: "none",
      },
    };

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
    getStatusSnapshotMock.mockResolvedValue(snapshot);

    const pending = useRepository().openRepo("/repo-b");
    await Promise.resolve();
    await Promise.resolve();

    enqueueStatusPatch(
      patch({
        generation: 2,
        baseRevision: 0,
        revision: 4,
        upserts: [entry("from-b.ts")],
        metadata: snapshot.metadata,
      })
    );

    resolveOpen({ root: "/repo-b", headBranch: "main" });
    await pending;
    await Promise.resolve();
    await Promise.resolve();

    const files = statusStore
      .getState()
      .groups.flatMap((group) => group.files)
      .map((file) => file.path);
    expect(files).toEqual(["from-b.ts"]);
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("from-b.ts");
    expect(getStatusSnapshotMock).toHaveBeenCalledOnce();
  });
});

describe("recoverFromSnapshot", () => {
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
  const metadata = (root: string) => ({
    root,
    headBranch: "main",
    headCommit: "abc",
    ahead: 0,
    behind: 0,
    operationState: "none" as const,
  });

  beforeEach(() => {
    repositoryStore.setState({ status: null, error: null, operations: new Set() });
    resetStatusStore();
    getStatusMock.mockReset();
    getStatusSnapshotMock.mockReset();
  });

  it("does not apply a snapshot after a newer patch has advanced the store", async () => {
    beginRepositorySession();
    repositoryStore.getState().setIdentity({ root: "/repo", headBranch: "main" }, { reset: false });

    let resolveSnapshot!: (value: StatusSnapshot) => void;
    const snapshotGate = new Promise<StatusSnapshot>((resolve) => {
      resolveSnapshot = resolve;
    });
    getStatusMock.mockResolvedValue(undefined);
    getStatusSnapshotMock.mockImplementation(() => snapshotGate);

    const pending = recoverFromSnapshot();
    await Promise.resolve();
    await Promise.resolve();

    applyStatusPatch(
      patch({
        generation: 2,
        revision: 4,
        upserts: [entry("newer.ts")],
      })
    );

    resolveSnapshot({
      generation: 2,
      revision: 3,
      phase: "settled",
      entries: [entry("stale.ts")],
      metadata: metadata("/repo"),
    });
    await pending;

    const files = statusStore
      .getState()
      .groups.flatMap((group) => group.files)
      .map((file) => file.path);
    expect(files).toEqual(["newer.ts"]);
    expect(statusStore.getState().generation).toBe(2);
    expect(statusStore.getState().revision).toBe(4);
  });

  it("does not apply a snapshot after the repository session or root changes", async () => {
    beginRepositorySession();
    repositoryStore.getState().setIdentity({ root: "/repo-a", headBranch: "main" }, { reset: false });

    let resolveSnapshot!: (value: StatusSnapshot) => void;
    const snapshotGate = new Promise<StatusSnapshot>((resolve) => {
      resolveSnapshot = resolve;
    });
    getStatusMock.mockResolvedValue(undefined);
    getStatusSnapshotMock.mockImplementation(() => snapshotGate);

    const pending = recoverFromSnapshot();
    await Promise.resolve();
    await Promise.resolve();

    beginRepositorySession();
    repositoryStore.getState().setIdentity({ root: "/repo-b", headBranch: "main" }, { reset: false });
    applyStatusPatch(patch({ upserts: [entry("from-b.ts")] }));
    repositoryStore.getState().syncStatusGroups();

    resolveSnapshot({
      generation: 1,
      revision: 1,
      phase: "settled",
      entries: [entry("from-a.ts")],
      metadata: metadata("/repo-a"),
    });
    await pending;

    const files = statusStore
      .getState()
      .groups.flatMap((group) => group.files)
      .map((file) => file.path);
    expect(files).toEqual(["from-b.ts"]);
    expect(repositoryStore.getState().status?.root).toBe("/repo-b");
  });
});

describe("refreshStatus", () => {
  beforeEach(() => {
    repositoryStore.setState({ status: null, error: null, operations: new Set() });
    resetStatusStore();
    getStatusMock.mockReset();
    getStatusSnapshotMock.mockReset();
    refreshStatusMock.mockReset();
  });

  it("forces a baseline scan instead of getStatus", async () => {
    beginRepositorySession();
    repositoryStore.getState().setIdentity({ root: "/repo", headBranch: "main" }, { reset: false });
    refreshStatusMock.mockResolvedValue({
      generation: 3,
      revision: 1,
      phase: "settled",
      entries: [{ group: "workingTree", path: "fresh.ts", status: "modified", renamePath: null }],
      metadata: {
        root: "/repo",
        headBranch: "main",
        headCommit: "abc",
        ahead: 0,
        behind: 0,
        operationState: "none",
      },
    });

    await useRepository().refreshStatus();

    expect(refreshStatusMock).toHaveBeenCalledOnce();
    expect(getStatusMock).not.toHaveBeenCalled();
    expect(getStatusSnapshotMock).not.toHaveBeenCalled();
    const files = statusStore
      .getState()
      .groups.flatMap((group) => group.files)
      .map((file) => file.path);
    expect(files).toEqual(["fresh.ts"]);
  });
});
