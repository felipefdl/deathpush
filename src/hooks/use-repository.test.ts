import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import { recoverFromSnapshot, useRepository } from "./use-repository";
import type { Intent, SessionSnapshot } from "../lib/git-types";

const snapshot = (root: string, path: string): SessionSnapshot => ({
  sessionGeneration: 0,
  sessionRevision: 0,
  statusGeneration: 0,
  statusRevision: 0,
  repo: {
    root,
    headBranch: "main",
    headCommit: "abc",
    ahead: 0,
    behind: 0,
    operationState: "none",
    phase: "settled",
  },
  groups: [
    {
      kind: "workingTree",
      label: "Changes",
      files: [{ path, status: "modified", renamePath: null }],
    },
  ],
  selection: { file: null, commit: null },
  scm: { amendMode: false, commitMessage: "", fileFilter: "" },
  actions: {
    canCommit: false,
    commitLabel: "Commit",
    commitDestructive: false,
    canStageAll: true,
    canUnstageAll: false,
    canDiscardAll: true,
    discardAllDestructive: true,
    sync: { enabled: true, kind: "fetch", destructive: false },
    operation: { continue: false, abort: false, skip: false, abortDestructive: true },
  },
  lastCommit: null,
  branches: [],
  stashes: [],
  tags: [],
  commitLog: [],
  commitDetail: null,
  fileHistoryPath: null,
  error: null,
});

const { sessionIntentMock, getSessionSnapshotMock } = vi.hoisted(() => ({
  sessionIntentMock: vi.fn(),
  getSessionSnapshotMock: vi.fn(),
}));

vi.mock("../lib/tauri-commands", () => ({
  sessionIntent: sessionIntentMock,
  getSessionSnapshot: getSessionSnapshotMock,
}));

describe("openRepo", () => {
  beforeEach(() => {
    repositoryStore.setState({ status: null, error: null, operations: new Set() });
    sessionIntentMock.mockReset();
    getSessionSnapshotMock.mockReset();
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
  });

  it("opens a repository through a session intent and applies the snapshot", async () => {
    sessionIntentMock.mockResolvedValue({ kind: "snapshot", snapshot: snapshot("/test/project", "a.ts") });
    await useRepository().openRepo("/test/project");
    expect(sessionIntentMock).toHaveBeenCalledWith({ type: "openRepository", path: "/test/project" });
    expect(repositoryStore.getState().status?.root).toBe("/test/project");
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("a.ts");
  });
});

describe("recoverFromSnapshot", () => {
  it("applies a fetched session snapshot", async () => {
    getSessionSnapshotMock.mockResolvedValue(snapshot("/repo", "recovered.ts"));
    await recoverFromSnapshot();
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("recovered.ts");
  });
});

describe("refreshStatus", () => {
  it("sends a refreshStatus intent", async () => {
    sessionIntentMock.mockReset();
    sessionIntentMock.mockResolvedValue({ kind: "snapshot", snapshot: snapshot("/repo", "fresh.ts") });
    await useRepository().refreshStatus();
    const intent = sessionIntentMock.mock.calls[0][0] as Intent;
    expect(intent).toEqual({ type: "refreshStatus" });
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("fresh.ts");
  });
});
