import { beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../stores/repository-store";
import type { DiffPayload, FileBlame, SessionActions, SessionSnapshot, SessionStatusEvent } from "./git-types";
import {
  acceptedBlame,
  acceptedDiff,
  applySessionSnapshot,
  applySessionStatus,
  sendDestructiveIntent,
  sendIntent,
} from "./session-client";

const { sessionIntentMock, confirmMock } = vi.hoisted(() => ({
  sessionIntentMock: vi.fn(),
  confirmMock: vi.fn(),
}));

vi.mock("./tauri-commands", () => ({
  sessionIntent: sessionIntentMock,
  getSessionSnapshot: vi.fn(),
}));
vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: confirmMock,
}));

const zeroActions: SessionActions = {
  canCommit: false,
  commitLabel: "Commit",
  commitDestructive: false,
  canStageAll: false,
  canUnstageAll: false,
  canDiscardAll: false,
  discardAllDestructive: true,
  sync: { enabled: true, kind: "fetch", destructive: false },
  operation: { continue: false, abort: false, skip: false, abortDestructive: true },
};

const snapshot = (root: string): SessionSnapshot => ({
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
      files: [{ path: "a.ts", status: "modified", renamePath: null }],
    },
  ],
  selection: { file: null, commit: null },
  scm: { amendMode: false, commitMessage: "", fileFilter: "" },
  actions: zeroActions,
  lastCommit: null,
  branches: [],
  stashes: [],
  tags: [],
  commitLog: [],
  commitDetail: null,
  fileHistoryPath: null,
  error: null,
});

const settledRepo = {
  root: "/repo",
  headBranch: "main",
  headCommit: "def",
  ahead: 1,
  behind: 0,
  operationState: "none" as const,
  phase: "settled" as const,
};

const dummyDiff = (): DiffPayload => ({
  path: "a.ts",
  original: "a",
  modified: "b",
  language: "typescript",
  fileType: "text",
  hunks: [],
  presence: { oldExists: true, newExists: true },
  editable: true,
  enableLineSelection: true,
  staged: false,
  contentHash: "hash",
});

const dummyBlame = (): FileBlame => ({
  path: "a.ts",
  lineGroups: [],
});


describe("sendIntent", () => {
  beforeEach(() => {
    repositoryStore.setState({
      status: null,
      commitMessage: "local",
      actions: null,
      selectedLoadId: 0,
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      amendMode: false,
      fileFilter: "",
      commitLog: [],
      selectedCommit: null,
      commitDetail: null,
      fileHistoryPath: null,
      sessionGeneration: 0,
      sessionRevision: 0,
    });
    sessionIntentMock.mockReset();
    confirmMock.mockReset();
  });

  it("does not replace groups on ack", async () => {
    sessionIntentMock.mockResolvedValue({ kind: "ack" });
    await sendIntent({ type: "setFileFilter", filter: "x" });
    expect(repositoryStore.getState().commitMessage).toBe("local");
    expect(repositoryStore.getState().status).toBeNull();
  });

  it("applies actions from a commit-message patch", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 1,
      patch: { kind: "actions", actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" } },
    });
    await sendIntent({ type: "setCommitMessage", message: "wip" });
    expect(repositoryStore.getState().actions?.canCommit).toBe(true);
    expect(repositoryStore.getState().commitMessage).toBe("local");
  });

  it("applies a returned snapshot and does not require a session:snapshot event", async () => {
    sessionIntentMock.mockResolvedValue({ kind: "snapshot", snapshot: snapshot("/repo") });
    await sendIntent({ type: "refreshStatus" });
    expect(repositoryStore.getState().status?.root).toBe("/repo");
  });

  it("applies scm patches without bumping selectedLoadId", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 1,
      patch: {
        kind: "scm",
        scm: { amendMode: true, commitMessage: "initial", fileFilter: "" },
        actions: { ...zeroActions, commitDestructive: true },
      },
    });
    await sendIntent({ type: "setAmend", enabled: true });
    expect(repositoryStore.getState().amendMode).toBe(true);
    expect(repositoryStore.getState().commitMessage).toBe("initial");
    expect(repositoryStore.getState().selectedLoadId).toBe(0);
  });

  it("applies fileHistory patches without bumping selectedLoadId", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 1,
      patch: {
        kind: "fileHistory",
        path: "README.md",
        commitLog: [
          {
            id: "abc",
            shortId: "abc",
            message: "initial",
            authorName: "Test",
            authorEmail: "test@example.com",
            authorDate: "0",
            parentIds: [],
            avatarUrl: "",
          },
        ],
      },
    });
    await sendIntent({ type: "openFileHistory", path: "README.md" });
    expect(repositoryStore.getState().fileHistoryPath).toBe("README.md");
    expect(repositoryStore.getState().commitLog).toHaveLength(1);
    expect(repositoryStore.getState().selectedLoadId).toBe(0);
  });

  it("drops a Diff from an older generation", async () => {
    repositoryStore.setState({
      sessionGeneration: 1,
      sessionRevision: 0,
      status: { ...settledRepo, groups: [] },
      diff: null,
    });
    sessionIntentMock.mockResolvedValue({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 4,
      payload: dummyDiff(),
    });
    const result = await sendIntent({ type: "openScmDiff", path: "a.ts", staged: false });
    expect(result.kind).toBe("diff");
    expect(repositoryStore.getState().diff).toBeNull();
    expect(repositoryStore.getState().sessionGeneration).toBe(1);
  });

  it("drops a Diff when the store root changed in flight", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 2,
      status: { ...settledRepo, root: "/old", groups: [] },
      diff: null,
    });
    sessionIntentMock.mockImplementation(async () => {
      repositoryStore.setState({
        sessionGeneration: 0,
        sessionRevision: 2,
        status: { ...settledRepo, root: "/new", groups: [] },
      });
      return {
        kind: "diff",
        sessionGeneration: 0,
        sessionRevision: 3,
        payload: dummyDiff(),
      };
    });
    const result = await sendIntent({ type: "openScmDiff", path: "a.ts", staged: false });
    expect(result.kind).toBe("diff");
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("advances the watermark from a stamped Ack and clears the current file", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 1,
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      diff: { path: "keep.ts", original: "a", modified: "b", fileType: "text" },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "ack",
      sessionGeneration: 0,
      sessionRevision: 2,
    });
    await sendIntent({ type: "clearFile" });
    expect(repositoryStore.getState().sessionRevision).toBe(2);
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("resends after NeedsConfirmation", async () => {
    confirmMock.mockResolvedValue(true);
    sessionIntentMock
      .mockResolvedValueOnce({ kind: "needsConfirmation", action: "deleteFile", message: "Move to trash?" })
      .mockResolvedValueOnce({ kind: "snapshot", snapshot: snapshot("/repo") });
    await sendDestructiveIntent({ type: "deleteFile", path: "a.ts", confirmed: false });
    expect(sessionIntentMock).toHaveBeenNthCalledWith(2, { type: "deleteFile", path: "a.ts", confirmed: true });
  });

  it("does not rewrite a rejected Diff as ack", async () => {
    repositoryStore.setState({
      sessionGeneration: 1,
      sessionRevision: 0,
      status: { ...settledRepo, groups: [] },
      diff: null,
    });
    const payload = dummyDiff();
    sessionIntentMock.mockResolvedValue({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 4,
      payload,
    });
    const result = await sendIntent({ type: "openScmDiff", path: "a.ts", staged: false });
    expect(result).toEqual({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 4,
      payload,
    });
    expect(repositoryStore.getState().diff).toBeNull();
  });

  it("does not rewind the watermark from a same-generation older Diff", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 5,
      status: { ...settledRepo, groups: [] },
      diff: null,
    });
    const payload = dummyDiff();
    sessionIntentMock.mockResolvedValue({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 3,
      payload,
    });
    const result = await sendIntent({ type: "openScmDiff", path: "a.ts", staged: false });
    expect(result).toEqual({
      kind: "diff",
      sessionGeneration: 0,
      sessionRevision: 3,
      payload,
    });
    expect(repositoryStore.getState().sessionRevision).toBe(5);
    expect(acceptedDiff(result)).toBe(false);
  });

  it("does not rewind the watermark from a same-generation older Blame", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 5,
      status: { ...settledRepo, groups: [] },
      blame: null,
    });
    const payload = dummyBlame();
    sessionIntentMock.mockResolvedValue({
      kind: "blame",
      sessionGeneration: 0,
      sessionRevision: 2,
      payload,
    });
    const result = await sendIntent({ type: "openBlame", path: "a.ts" });
    expect(result.kind).toBe("blame");
    expect(repositoryStore.getState().sessionRevision).toBe(5);
    expect(acceptedBlame(result)).toBe(false);
  });

  it("increments selectedLoadId on ClearFile Ack so late diffs cannot match", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 1,
      selectedLoadId: 4,
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      diff: { path: "keep.ts", original: "a", modified: "b", fileType: "text" },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "ack",
      sessionGeneration: 0,
      sessionRevision: 2,
    });
    await sendIntent({ type: "clearFile" });
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().selectedLoadId).toBe(5);
  });

  it("increments selectedLoadId when a snapshot clears the file", () => {
    repositoryStore.setState({
      selectedLoadId: 7,
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      sessionGeneration: 0,
      sessionRevision: 0,
    });
    applySessionSnapshot(snapshot("/repo"));
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().selectedLoadId).toBe(8);
  });
});

describe("acceptedDiff", () => {
  beforeEach(() => {
    repositoryStore.setState({
      sessionGeneration: 1,
      sessionRevision: 4,
      status: { ...settledRepo, groups: [] },
    });
  });

  it("accepts a Diff stamped with the current generation and revision", () => {
    expect(
      acceptedDiff({
        kind: "diff",
        sessionGeneration: 1,
        sessionRevision: 4,
        payload: dummyDiff(),
      })
    ).toBe(true);
  });

  it("rejects a Diff from an older generation", () => {
    expect(
      acceptedDiff({
        kind: "diff",
        sessionGeneration: 0,
        sessionRevision: 9,
        payload: dummyDiff(),
      })
    ).toBe(false);
  });

  it("rejects a same-generation older Diff", () => {
    expect(
      acceptedDiff({
        kind: "diff",
        sessionGeneration: 1,
        sessionRevision: 3,
        payload: dummyDiff(),
      })
    ).toBe(false);
  });

  it("rejects a Blame payload", () => {
    expect(
      acceptedDiff({
        kind: "blame",
        sessionGeneration: 1,
        sessionRevision: 4,
        payload: dummyBlame(),
      })
    ).toBe(false);
  });

  it("accepts a Blame stamped with the current generation and revision", () => {
    expect(
      acceptedBlame({
        kind: "blame",
        sessionGeneration: 1,
        sessionRevision: 4,
        payload: dummyBlame(),
      })
    ).toBe(true);
  });

  it("rejects Blame from an older generation", () => {
    expect(
      acceptedBlame({
        kind: "blame",
        sessionGeneration: 0,
        sessionRevision: 4,
        payload: dummyBlame(),
      })
    ).toBe(false);
  });
});



describe("applySessionStatus", () => {
  beforeEach(() => {
    repositoryStore.setState({
      status: null,
      selectedFile: null,
      diff: null,
      actions: null,
      lastCommit: null,
      branches: [],
      tags: [],
      commitLog: [],
      stashes: [],
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 0,
    });
    sessionIntentMock.mockReset();
  });
  it("updates groups and actions without replacing scm text", () => {
    repositoryStore.setState({
      commitMessage: "keep me",
      amendMode: true,
      selectedLoadId: 4,
      selectedFile: { path: "a.ts", staged: false, groupKind: "workingTree" },
      actions: null,
      status: null,
      sessionGeneration: 0,
      sessionRevision: 0,
    });
    const event: SessionStatusEvent = {
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "a.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canStageAll: true },
      selection: { file: { path: "a.ts", staged: false, groupKind: "workingTree" }, commit: null },
    };
    applySessionStatus(event);
    expect(repositoryStore.getState().status?.groups[0]?.files[0]?.path).toBe("a.ts");
    expect(repositoryStore.getState().actions?.canStageAll).toBe(true);
    expect(repositoryStore.getState().commitMessage).toBe("keep me");
    expect(repositoryStore.getState().amendMode).toBe(true);
    expect(repositoryStore.getState().selectedLoadId).toBe(4);
  });

  it("increments selectedLoadId when status clears the file", () => {
    repositoryStore.setState({
      selectedLoadId: 4,
      selectedFile: { path: "a.ts", staged: false, groupKind: "workingTree" },
      sessionGeneration: 0,
      sessionRevision: 0,
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [],
      actions: zeroActions,
      selection: { file: null, commit: null },
    });
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().selectedLoadId).toBe(5);
  });


  it("does not overwrite history extras on an ordinary status event", () => {
    repositoryStore.setState({
      lastCommit: { shortId: "old", message: "keep last", authorDate: "0" },
      branches: [{ name: "keep-branch", isHead: true, isRemote: false, upstream: null, ahead: 0, behind: 0 }],
      tags: [{ name: "keep-tag", message: null, targetId: "old", isAnnotated: false }],
      commitLog: [
        {
          id: "old",
          shortId: "old",
          message: "keep log",
          authorName: "Test",
          authorEmail: "test@example.com",
          authorDate: "0",
          parentIds: [],
          avatarUrl: "",
        },
      ],
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [],
      actions: zeroActions,
      selection: { file: null, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.lastCommit?.message).toBe("keep last");
    expect(state.branches[0]?.name).toBe("keep-branch");
    expect(state.tags[0]?.name).toBe("keep-tag");
    expect(state.commitLog[0]?.message).toBe("keep log");
  });

  it("applies compact extras when head metadata changes", () => {
    repositoryStore.setState({
      lastCommit: { shortId: "old", message: "stale", authorDate: "0" },
      branches: [{ name: "stale", isHead: true, isRemote: false, upstream: null, ahead: 0, behind: 0 }],
      tags: [],
      commitLog: [],
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 1,
      repo: {
        root: "/repo",
        headBranch: "feature",
        headCommit: "abc",
        ahead: 0,
        behind: 0,
        operationState: "none",
        phase: "settled",
      },
      groups: [],
      actions: zeroActions,
      selection: { file: null, commit: null },
      extras: {
        lastCommit: { shortId: "abc", message: "new head", authorDate: "1" },
        branches: [{ name: "feature", isHead: true, isRemote: false, upstream: null, ahead: 0, behind: 0 }],
        tags: [{ name: "v1", message: null, targetId: "abc", isAnnotated: false }],
        commitLog: [
          {
            id: "abc",
            shortId: "abc",
            message: "new head",
            authorName: "Test",
            authorEmail: "test@example.com",
            authorDate: "1",
            parentIds: [],
            avatarUrl: "",
          },
        ],
      },
    });
    const state = repositoryStore.getState();
    expect(state.lastCommit?.message).toBe("new head");
    expect(state.branches[0]?.name).toBe("feature");
    expect(state.tags[0]?.name).toBe("v1");
    expect(state.commitLog[0]?.message).toBe("new head");
  });

  it("keeps a newer patch's actions when an older status event arrives later", async () => {
    repositoryStore.setState({
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      sessionGeneration: 0,
      sessionRevision: 0,
      actions: null,
    });
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 2,
      patch: { kind: "actions", actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" } },
    });
    await sendIntent({ type: "setCommitMessage", message: "wip" });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "b.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" },
      selection: { file: null, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.actions?.canCommit).toBe(true);
    expect(state.status?.groups[0]?.files[0]?.path).toBe("b.ts");
    expect(state.sessionRevision).toBe(2);
    expect(state.selectedFile?.path).toBe("keep.ts");
  });

  it("lets a status event apply before a later patch", async () => {
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "b.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: false, canStageAll: true },
      selection: { file: { path: "b.ts", staged: false, groupKind: "workingTree" }, commit: null },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 2,
      patch: { kind: "actions", actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" } },
    });
    await sendIntent({ type: "setCommitMessage", message: "wip" });
    const state = repositoryStore.getState();
    expect(state.actions?.canCommit).toBe(true);
    expect(state.actions?.canStageAll).toBe(false);
    expect(state.status?.groups[0]?.files[0]?.path).toBe("b.ts");
    expect(state.sessionRevision).toBe(2);
  });

  it("applies session-derived status at the same revision as the last patch", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 2,
      patch: { kind: "actions", actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" } },
    });
    await sendIntent({ type: "setCommitMessage", message: "wip" });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 2,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "c.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: true, canStageAll: true },
      selection: { file: { path: "c.ts", staged: false, groupKind: "workingTree" }, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.actions?.canCommit).toBe(true);
    expect(state.actions?.canStageAll).toBe(true);
    expect(state.selectedFile?.path).toBe("c.ts");
    expect(state.status?.groups[0]?.files[0]?.path).toBe("c.ts");
  });

  it("keeps newer snapshot session state when an older status event arrives", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "snapshot",
      snapshot: {
        ...snapshot("/repo"),
        sessionGeneration: 0,
        sessionRevision: 4,
        statusGeneration: 0,
        statusRevision: 8,
        actions: { ...zeroActions, canCommit: true, commitLabel: "Undo Commit" },
        selection: { file: { path: "keep.ts", staged: false, groupKind: "workingTree" }, commit: null },
      },
    });
    await sendIntent({ type: "refreshStatus" });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 3,
      statusGeneration: 0,
      statusRevision: 7,
      repo: { ...settledRepo, ahead: 2 },
      groups: [
        {
          kind: "index",
          label: "Staged",
          files: [{ path: "d.ts", status: "added", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" },
      selection: { file: null, commit: null },
      extras: {
        lastCommit: { shortId: "old", message: "stale extras", authorDate: "0" },
        branches: [],
        tags: [],
        commitLog: [],
      },
    });
    const state = repositoryStore.getState();
    expect(state.actions?.commitLabel).toBe("Undo Commit");
    expect(state.actions?.canCommit).toBe(true);
    expect(state.selectedFile?.path).toBe("keep.ts");
    expect(state.lastCommit).toBeNull();
    expect(state.status?.ahead).toBe(0);
    expect(state.status?.groups[0]?.files[0]?.path).toBe("a.ts");
    expect(state.sessionRevision).toBe(4);
  });

  it("does not apply an older patch over a newer snapshot", async () => {
    sessionIntentMock.mockResolvedValueOnce({
      kind: "snapshot",
      snapshot: {
        ...snapshot("/repo"),
        sessionGeneration: 0,
        sessionRevision: 5,
        actions: { ...zeroActions, canCommit: true, commitLabel: "Undo Commit" },
      },
    });
    await sendIntent({ type: "refreshStatus" });
    sessionIntentMock.mockResolvedValueOnce({
      kind: "patch",
      sessionGeneration: 0,
      sessionRevision: 4,
      patch: { kind: "actions", actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" } },
    });
    await sendIntent({ type: "setCommitMessage", message: "late" });
    const state = repositoryStore.getState();
    expect(state.actions?.commitLabel).toBe("Undo Commit");
    expect(state.sessionRevision).toBe(5);
  });

  it("replaces a high-revision repo A session with a new-generation repo B snapshot", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 40,
      commitMessage: "from-A",
      actions: { ...zeroActions, canCommit: true, commitLabel: "Undo Commit" },
      status: {
        root: "/repo-a",
        headBranch: "main",
        headCommit: "aaa",
        ahead: 0,
        behind: 0,
        groups: [
          { kind: "workingTree", label: "Changes", files: [{ path: "a.ts", status: "modified", renamePath: null }] },
        ],
        operationState: "none",
      },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "snapshot",
      snapshot: {
        ...snapshot("/repo-b"),
        sessionGeneration: 1,
        sessionRevision: 1,
        scm: { amendMode: false, commitMessage: "from-B", fileFilter: "" },
        actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" },
      },
    });
    await sendIntent({ type: "openRepository", path: "/repo-b" });
    const state = repositoryStore.getState();
    expect(state.status?.root).toBe("/repo-b");
    expect(state.commitMessage).toBe("from-B");
    expect(state.actions?.commitLabel).toBe("Commit");
    expect(state.sessionGeneration).toBe(1);
    expect(state.sessionRevision).toBe(1);
  });

  it("ignores a queued old-generation status event after a repo switch", async () => {
    repositoryStore.setState({
      sessionGeneration: 1,
      sessionRevision: 1,
      commitMessage: "from-B",
      actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" },
      status: {
        root: "/repo-b",
        headBranch: "main",
        headCommit: "bbb",
        ahead: 0,
        behind: 0,
        groups: [
          { kind: "workingTree", label: "Changes", files: [{ path: "b.ts", status: "modified", renamePath: null }] },
        ],
        operationState: "none",
      },
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 99,
      statusGeneration: 0,
      statusRevision: 1,
      repo: { ...settledRepo, root: "/repo-a" },
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "a-old.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: true, commitLabel: "Undo Commit" },
      selection: { file: null, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.status?.root).toBe("/repo-b");
    expect(state.status?.groups[0]?.files[0]?.path).toBe("b.ts");
    expect(state.commitMessage).toBe("from-B");
    expect(state.actions?.canCommit).toBe(false);
    expect(state.sessionGeneration).toBe(1);
  });

  it("lets a new-generation snapshot fill session state after an earlier status event", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 12,
      commitMessage: "from-A",
      actions: { ...zeroActions, canCommit: true, commitLabel: "Undo Commit" },
    });
    applySessionStatus({
      sessionGeneration: 1,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 1,
      repo: { ...settledRepo, root: "/repo-b" },
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "b.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canStageAll: true },
      selection: { file: null, commit: null },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "snapshot",
      snapshot: {
        ...snapshot("/repo-b"),
        sessionGeneration: 1,
        sessionRevision: 1,
        scm: { amendMode: false, commitMessage: "from-B", fileFilter: "" },
        actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" },
      },
    });
    await sendIntent({ type: "openRepository", path: "/repo-b" });
    const state = repositoryStore.getState();
    expect(state.status?.root).toBe("/repo-b");
    expect(state.commitMessage).toBe("from-B");
    expect(state.actions?.canCommit).toBe(true);
    expect(state.sessionGeneration).toBe(1);
    expect(state.sessionRevision).toBe(1);
  });

  it("applies newer status groups when session revision is older", () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 5,
      statusGeneration: 0,
      statusRevision: 1,
      selectedFile: { path: "keep.ts", staged: false, groupKind: "workingTree" },
      actions: { ...zeroActions, canCommit: true, commitLabel: "Commit" },
      status: { ...settledRepo, groups: [] },
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 4,
      statusGeneration: 0,
      statusRevision: 3,
      repo: settledRepo,
      groups: [
        {
          kind: "workingTree",
          label: "Changes",
          files: [{ path: "new.ts", status: "modified", renamePath: null }],
        },
      ],
      actions: { ...zeroActions, canCommit: false, commitLabel: "Commit" },
      selection: { file: null, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.status?.groups[0]?.files[0]?.path).toBe("new.ts");
    expect(state.statusRevision).toBe(3);
    expect(state.sessionRevision).toBe(5);
    expect(state.actions?.canCommit).toBe(true);
    expect(state.selectedFile?.path).toBe("keep.ts");
  });

  it("does not apply older status groups over a newer snapshot", async () => {
    sessionIntentMock.mockResolvedValue({
      kind: "snapshot",
      snapshot: {
        ...snapshot("/repo"),
        sessionGeneration: 0,
        sessionRevision: 4,
        statusGeneration: 0,
        statusRevision: 8,
        groups: [
          {
            kind: "workingTree",
            label: "Changes",
            files: [{ path: "snap.ts", status: "modified", renamePath: null }],
          },
        ],
      },
    });
    await sendIntent({ type: "refreshStatus" });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 3,
      statusGeneration: 0,
      statusRevision: 7,
      repo: { ...settledRepo, ahead: 2 },
      groups: [
        {
          kind: "index",
          label: "Staged",
          files: [{ path: "old.ts", status: "added", renamePath: null }],
        },
      ],
      actions: zeroActions,
      selection: { file: null, commit: null },
    });
    const state = repositoryStore.getState();
    expect(state.status?.groups[0]?.files[0]?.path).toBe("snap.ts");
    expect(state.statusRevision).toBe(8);
    expect(state.sessionRevision).toBe(4);
  });

  it("does not clear lastCommit when extras only has branches", () => {
    repositoryStore.setState({
      lastCommit: { shortId: "keep", message: "keep last", authorDate: "0" },
      branches: [{ name: "old", isHead: true, isRemote: false, upstream: null, ahead: 0, behind: 0 }],
      sessionGeneration: 0,
      sessionRevision: 0,
      statusGeneration: 0,
      statusRevision: 0,
    });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 1,
      repo: settledRepo,
      groups: [],
      actions: zeroActions,
      selection: { file: null, commit: null },
      extras: {
        branches: [{ name: "feature", isHead: true, isRemote: false, upstream: null, ahead: 0, behind: 0 }],
      },
    });
    const state = repositoryStore.getState();
    expect(state.lastCommit?.message).toBe("keep last");
    expect(state.branches[0]?.name).toBe("feature");
  });

  it("does not resurrect a local deselect from equal-revision status before ClearFile Ack", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 1,
      selectedFile: { path: "a.ts", staged: false, groupKind: "workingTree" },
      diff: { path: "a.ts", original: "a", modified: "b", fileType: "text" },
      status: { ...settledRepo, groups: [] },
    });
    const ack = Promise.withResolvers<{
      kind: "ack";
      sessionGeneration: number;
      sessionRevision: number;
    }>();
    sessionIntentMock.mockReturnValue(ack.promise);
    const pending = sendIntent({ type: "clearFile" });
    repositoryStore.setState({ selectedFile: null, diff: null });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 2,
      repo: settledRepo,
      groups: [],
      actions: zeroActions,
      selection: { file: { path: "a.ts", staged: false, groupKind: "workingTree" }, commit: null },
    });
    expect(repositoryStore.getState().selectedFile).toBeNull();
    ack.resolve({ kind: "ack", sessionGeneration: 0, sessionRevision: 2 });
    await pending;
    expect(repositoryStore.getState().selectedFile).toBeNull();
  });

  it("keeps selection cleared after ClearFile Ack when older status arrives", async () => {
    repositoryStore.setState({
      sessionGeneration: 0,
      sessionRevision: 1,
      selectedFile: { path: "a.ts", staged: false, groupKind: "workingTree" },
      status: { ...settledRepo, groups: [] },
    });
    sessionIntentMock.mockResolvedValue({
      kind: "ack",
      sessionGeneration: 0,
      sessionRevision: 2,
    });
    await sendIntent({ type: "clearFile" });
    applySessionStatus({
      sessionGeneration: 0,
      sessionRevision: 1,
      statusGeneration: 0,
      statusRevision: 3,
      repo: settledRepo,
      groups: [],
      actions: zeroActions,
      selection: { file: { path: "a.ts", staged: false, groupKind: "workingTree" }, commit: null },
    });
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(repositoryStore.getState().sessionRevision).toBe(2);
  });
});
