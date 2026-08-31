import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import { useRepository } from "./use-repository";

const basicStatus = {
  root: "/test/project",
  headBranch: "main",
  headCommit: "abc",
  ahead: 0,
  behind: 0,
  groups: [],
  operationState: "none" as const,
};

const fullStatus = { ...basicStatus, groups: [{ id: "working", label: "Changes", files: [] }] };

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
    vi.stubGlobal("requestAnimationFrame", (cb: FrameRequestCallback) => {
      cb(0);
      return 1;
    });
    openRepositoryMock.mockReset();
    getStatusMock.mockReset();
  });

  it("shows repository identity while full status is pending", async () => {
    let resolveOpen!: (value: typeof basicStatus) => void;
    let resolveStatus!: (value: typeof fullStatus) => void;
    const openGate = new Promise<typeof basicStatus>((resolve) => {
      resolveOpen = resolve;
    });
    const statusGate = new Promise<typeof fullStatus>((resolve) => {
      resolveStatus = resolve;
    });
    openRepositoryMock.mockImplementation(() => openGate);
    getStatusMock.mockImplementation(() => statusGate);

    const pending = useRepository().openRepo("/test/project");
    await Promise.resolve();

    expect(repositoryStore.getState().operations.has("open-repo")).toBe(true);
    expect(repositoryStore.getState().status).toBeNull();

    resolveOpen(basicStatus);
    await Promise.resolve();
    await Promise.resolve();

    expect(repositoryStore.getState().status).toEqual(basicStatus);
    expect(getStatusMock).toHaveBeenCalledOnce();
    expect(repositoryStore.getState().operations.has("open-repo")).toBe(true);

    resolveStatus(fullStatus);
    await pending;

    expect(repositoryStore.getState().status).toEqual(fullStatus);
    expect(repositoryStore.getState().operations.has("open-repo")).toBe(false);
  });
});
