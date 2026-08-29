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

  it("keeps the welcome screen up until full status is ready", async () => {
    const openGate = Promise.withResolvers<typeof basicStatus>();
    const statusGate = Promise.withResolvers<typeof fullStatus>();
    openRepositoryMock.mockImplementation(() => openGate.promise);
    getStatusMock.mockImplementation(() => statusGate.promise);


    const pending = useRepository().openRepo("/test/project");
    await Promise.resolve();

    expect(repositoryStore.getState().operations.has("open-repo")).toBe(true);
    expect(repositoryStore.getState().status).toBeNull();

    openGate.resolve(basicStatus);
    await Promise.resolve();
    await Promise.resolve();

    expect(repositoryStore.getState().status).toBeNull();
    expect(repositoryStore.getState().operations.has("open-repo")).toBe(true);

    statusGate.resolve(fullStatus);
    await pending;

    expect(repositoryStore.getState().status).toEqual(fullStatus);
    expect(repositoryStore.getState().operations.has("open-repo")).toBe(false);
  });
});
