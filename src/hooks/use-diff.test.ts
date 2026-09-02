import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import type { Intent } from "../lib/git-types";
import { repositoryStore } from "../stores/repository-store";
import { useDiff } from "./use-diff";
import { loadScmDiffSources } from "../components/pierre/pierre-file-diff";
import { clearScmDiffPayload } from "../lib/pierre/scm-diff-payload";

const { sendIntentMock } = vi.hoisted(() => ({
  sendIntentMock: vi.fn(),
}));

vi.mock("../lib/session-client", async (importOriginal) => {
  const actual = await importOriginal();
  return { ...actual, sendIntent: sendIntentMock };
});


const diffPayload = {
  path: "src/a.ts",
  original: "old",
  modified: "new",
  language: "typescript",
  fileType: "text",
  hunks: [],
  presence: { oldExists: true, newExists: true },
  editable: true,
  enableLineSelection: true,
  staged: false,
  contentHash: "hash-new",
};

type DiffOutcome = {
  kind: "diff";
  sessionGeneration: number;
  sessionRevision: number;
  payload: typeof diffPayload;
};

const diffOutcome = (payload: typeof diffPayload = diffPayload): DiffOutcome => ({
  kind: "diff",
  sessionGeneration: 0,
  sessionRevision: 0,
  payload,
});

const repoStatus = {
  root: "/repo",
  headBranch: "main",
  headCommit: "abc",
  ahead: 0,
  behind: 0,
  groups: [] as const,
  operationState: "none" as const,
};


describe("loadDiff", () => {
  beforeEach(() => {
    repositoryStore.setState({
      selectedFile: null,
      selectedLoadId: 0,
      diff: null,
      diffLoadId: null,
      error: null,
      sessionGeneration: 0,
      sessionRevision: 0,
      status: repoStatus,
    });
    clearScmDiffPayload();
    sendIntentMock.mockReset();
  });

  it("requests the scm diff without waiting for selectFile snapshot work", async () => {
    const blocked = Promise.withResolvers<never>();
    sendIntentMock.mockImplementation((intent: Intent) => {
      if (intent.type === "openScmDiff") {
        return Promise.resolve(diffOutcome());
      }
      return blocked.promise;
    });

    void useDiff().loadDiff("src/a.ts", false, "untracked");
    await Promise.resolve();

    expect(sendIntentMock).toHaveBeenCalledWith({
      type: "openScmDiff",
      path: "src/a.ts",
      staged: false,
      groupKind: "untracked",
    });
    expect(sendIntentMock.mock.calls.map(([intent]) => (intent as Intent).type)).not.toContain("selectFile");
    expect(repositoryStore.getState().selectedFile).toEqual({
      path: "src/a.ts",
      staged: false,
      groupKind: "untracked",
    });
  });

  it("applies the scm diff while selectFile snapshot work is still blocked", async () => {
    const blocked = Promise.withResolvers<never>();
    sendIntentMock.mockImplementation((intent: Intent) => {
      if (intent.type === "openScmDiff") {
        return Promise.resolve(diffOutcome({ ...diffPayload, staged: true }));
      }
      return blocked.promise;
    });

    void useDiff().loadDiff("src/a.ts", true, "index");
    await Promise.resolve();
    await Promise.resolve();

    expect(repositoryStore.getState().selectedFile).toEqual({
      path: "src/a.ts",
      staged: true,
      groupKind: "index",
    });
    expect(repositoryStore.getState().diff).toEqual({
      path: "src/a.ts",
      original: "old",
      modified: "new",
      originalLanguage: "typescript",
      fileType: "text",
    });
  });

  it("coalesces concurrent loads of the same file into one openScmDiff", async () => {
    const gate = Promise.withResolvers<DiffOutcome>();
    sendIntentMock.mockImplementation(() => gate.promise);

    const { loadDiff } = useDiff();
    try {
      void loadDiff("src/a.ts", false, "workingTree");
      void loadDiff("src/a.ts", false, "workingTree");
      await Promise.resolve();

      expect(sendIntentMock).toHaveBeenCalledTimes(1);

      gate.resolve(diffOutcome());
      await Promise.resolve();
      await Promise.resolve();

      expect(repositoryStore.getState().diff?.path).toBe("src/a.ts");
    } finally {
      gate.resolve(diffOutcome());
      await Promise.resolve();
      await Promise.resolve();
    }
  });

  it("reloads the same file after the in-flight load settles", async () => {
    sendIntentMock.mockResolvedValue(diffOutcome());
    const { loadDiff } = useDiff();
    await loadDiff("src/a.ts", false, "workingTree");
    await loadDiff("src/a.ts", false, "workingTree");
    expect(sendIntentMock).toHaveBeenCalledTimes(2);
  });

  it("does not coalesce loads that differ by group", async () => {
    const working = Promise.withResolvers<DiffOutcome>();
    const index = Promise.withResolvers<DiffOutcome>();
    sendIntentMock.mockImplementationOnce(() => working.promise).mockImplementationOnce(() => index.promise);
    const { loadDiff } = useDiff();
    try {
      void loadDiff("src/a.ts", false, "workingTree");
      void loadDiff("src/a.ts", true, "index");
      await Promise.resolve();
      expect(sendIntentMock).toHaveBeenCalledTimes(2);
    } finally {
      working.resolve(diffOutcome());
      index.resolve(diffOutcome({ ...diffPayload, staged: true }));
      await Promise.resolve();
      await Promise.resolve();
    }
  });

  it("starts a fresh load when reselecting A while A's first request is still in flight after B", async () => {
    const aFirst = Promise.withResolvers<DiffOutcome>();
    const bGate = Promise.withResolvers<DiffOutcome>();
    const aSecond = Promise.withResolvers<DiffOutcome>();
    const bPayload = { ...diffPayload, path: "src/b.ts", original: "b-old", modified: "b-new" };
    let aCalls = 0;
    sendIntentMock.mockImplementation((intent: Intent) => {
      if (intent.type !== "openScmDiff") return Promise.resolve(diffOutcome());
      if (intent.path === "src/a.ts") {
        aCalls += 1;
        return aCalls === 1 ? aFirst.promise : aSecond.promise;
      }
      return bGate.promise;
    });

    const { loadDiff } = useDiff();
    try {
      void loadDiff("src/a.ts", false, "workingTree");
      void loadDiff("src/b.ts", false, "workingTree");
      void loadDiff("src/a.ts", false, "workingTree");
      await Promise.resolve();

      expect(sendIntentMock).toHaveBeenCalledTimes(3);
      expect(repositoryStore.getState().selectedFile).toEqual({
        path: "src/a.ts",
        staged: false,
        groupKind: "workingTree",
      });

      aFirst.resolve(diffOutcome());
      bGate.resolve(diffOutcome(bPayload));
      await Promise.resolve();
      await Promise.resolve();
      expect(repositoryStore.getState().diff).toBeNull();

      aSecond.resolve(diffOutcome());
      await Promise.resolve();
      await Promise.resolve();
      expect(repositoryStore.getState().diff?.path).toBe("src/a.ts");
    } finally {
      aFirst.resolve(diffOutcome());
      bGate.resolve(diffOutcome(bPayload));
      aSecond.resolve(diffOutcome());
      await Promise.resolve();
      await Promise.resolve();
    }
  });

  it("lets Pierre consume each file's openScmDiff payload once", async () => {
    const bPayload = { ...diffPayload, path: "src/b.ts" };
    sendIntentMock.mockResolvedValue(diffOutcome());
    await useDiff().loadDiff("src/a.ts", false, "workingTree");
    sendIntentMock.mockClear();

    await expect(
      loadScmDiffSources({
        path: "src/a.ts",
        staged: false,
        groupKind: "workingTree",
        loadId: repositoryStore.getState().diffLoadId ?? 0,
        consumeCache: true,
      })
    ).resolves.toEqual(diffPayload);
    expect(sendIntentMock).not.toHaveBeenCalled();

    sendIntentMock.mockResolvedValue(diffOutcome(bPayload));
    await useDiff().loadDiff("src/b.ts", false, "workingTree");
    sendIntentMock.mockClear();

    await expect(
      loadScmDiffSources({
        path: "src/b.ts",
        staged: false,
        groupKind: "workingTree",
        loadId: repositoryStore.getState().diffLoadId ?? 0,
        consumeCache: true,
      })
    ).resolves.toEqual(bPayload);
    expect(sendIntentMock).not.toHaveBeenCalled();
  });

  it("does not hand off a payload from a different groupKind", async () => {
    sendIntentMock.mockResolvedValue(diffOutcome());
    await useDiff().loadDiff("src/a.ts", false, "workingTree");
    sendIntentMock.mockClear();
    sendIntentMock.mockResolvedValue(diffOutcome());

    await expect(
      loadScmDiffSources({
        path: "src/a.ts",
        staged: false,
        groupKind: "untracked",
        loadId: repositoryStore.getState().diffLoadId ?? 0,
        consumeCache: true,
      })
    ).resolves.toEqual(diffPayload);
    expect(sendIntentMock).toHaveBeenCalledWith({
      type: "openScmDiff",
      path: "src/a.ts",
      staged: false,
      groupKind: "untracked",
    });
  });

  it("refetches on hunk or disk reload instead of consuming a leftover payload", async () => {
    sendIntentMock.mockResolvedValue(diffOutcome());
    await useDiff().loadDiff("src/a.ts", false, "workingTree");
    sendIntentMock.mockClear();
    sendIntentMock.mockResolvedValue(diffOutcome());

    await expect(
      loadScmDiffSources({
        path: "src/a.ts",
        staged: false,
        groupKind: "workingTree",
        loadId: repositoryStore.getState().diffLoadId ?? 0,
        consumeCache: false,
      })
    ).resolves.toEqual(diffPayload);
    expect(sendIntentMock).toHaveBeenCalledWith({
      type: "openScmDiff",
      path: "src/a.ts",
      staged: false,
      groupKind: "workingTree",
    });
  });

  it("does not apply a late diff after generation changes", async () => {
    const gate = Promise.withResolvers<DiffOutcome>();
    sendIntentMock.mockImplementation(() => gate.promise);
    void useDiff().loadDiff("src/a.ts", false, "workingTree");
    await Promise.resolve();
    repositoryStore.setState({ sessionGeneration: 1 });
    gate.resolve(diffOutcome());
    await Promise.resolve();
    await Promise.resolve();
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("does not apply a late diff after the repo root changes", async () => {
    const gate = Promise.withResolvers<DiffOutcome>();
    sendIntentMock.mockImplementation(() => gate.promise);
    void useDiff().loadDiff("src/a.ts", false, "workingTree");
    await Promise.resolve();
    repositoryStore.setState({ status: { ...repoStatus, root: "/other" } });
    gate.resolve(diffOutcome());
    await Promise.resolve();
    await Promise.resolve();
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("does not apply a late diff after clearing selection", async () => {
    const gate = Promise.withResolvers<DiffOutcome>();
    sendIntentMock.mockImplementation((intent: Intent) => {
      if (intent.type === "openScmDiff") return gate.promise;
      return Promise.resolve({ kind: "ack", sessionGeneration: 0, sessionRevision: 1 });
    });
    const { loadDiff, clearDiff } = useDiff();
    void loadDiff("src/a.ts", false, "workingTree");
    await Promise.resolve();
    const loadId = repositoryStore.getState().selectedLoadId;
    clearDiff();
    expect(repositoryStore.getState().selectedLoadId).toBeGreaterThan(loadId);
    gate.resolve(diffOutcome());
    await Promise.resolve();
    await Promise.resolve();
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("does not apply a same-generation older Diff", async () => {
    repositoryStore.setState({ sessionRevision: 4 });
    sendIntentMock.mockResolvedValue({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 2,
      payload: diffPayload,
    });
    await useDiff().loadDiff("src/a.ts", false, "workingTree");
    expect(repositoryStore.getState().diff).toBeNull();
  });

});
