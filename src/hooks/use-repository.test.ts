import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import { resetStatusStore, statusStore } from "../stores/status-store";
import { useRepository } from "./use-repository";

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
});
