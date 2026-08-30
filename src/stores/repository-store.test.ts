import { describe, it, expect, beforeEach } from "vite-plus/test";
import { repositoryStore } from "./repository-store";

beforeEach(() => {
  repositoryStore.setState({
    status: null,
    selectedFile: null,
    selectedLoadId: 0,
    diff: null,
    diffLoadId: null,
    branches: [],
    operations: new Set<string>(),
    error: null,
    stashes: [],
    amendMode: false,
    fileFilter: "",
    commitLog: [],
    selectedCommit: null,
    commitDetail: null,
    tags: [],
    terminalGroups: [],
    activeGroupId: null,
    terminalIdCounter: 0,
    isDiffDirty: false,
    blame: null,
    cursorLine: null,
  });
});

describe("repository store", () => {
  describe("initial state", () => {
    it("has correct default values", () => {
      const state = repositoryStore.getState();
      expect(state.status).toBeNull();
      expect(state.selectedFile).toBeNull();
      expect(state.selectedLoadId).toBe(0);
      expect(state.diff).toBeNull();
      expect(state.diffLoadId).toBeNull();
      expect(state.branches).toEqual([]);
      expect(state.error).toBeNull();
      expect(state.stashes).toEqual([]);
      expect(state.amendMode).toBe(false);
      expect(state.fileFilter).toBe("");
      expect(state.commitLog).toEqual([]);
      expect(state.selectedCommit).toBeNull();
      expect(state.commitDetail).toBeNull();
      expect(state.tags).toEqual([]);
      expect(state.terminalGroups).toEqual([]);
      expect(state.activeGroupId).toBeNull();
      expect(state.terminalIdCounter).toBe(0);
      expect(state.isDiffDirty).toBe(false);
      expect(state.blame).toBeNull();
      expect(state.cursorLine).toBeNull();
    });

    it("has an empty operations set", () => {
      const state = repositoryStore.getState();
      expect(state.operations).toBeInstanceOf(Set);
      expect(state.operations.size).toBe(0);
    });
  });

  describe("operations", () => {
    it("startOperation adds to the set", () => {
      repositoryStore.getState().startOperation("fetch");
      expect(repositoryStore.getState().operations.has("fetch")).toBe(true);
    });

    it("endOperation removes from the set", () => {
      repositoryStore.getState().startOperation("fetch");
      repositoryStore.getState().endOperation("fetch");
      expect(repositoryStore.getState().operations.has("fetch")).toBe(false);
    });

    it("isOperationRunning returns correct value", () => {
      expect(repositoryStore.getState().isOperationRunning("push")).toBe(false);
      repositoryStore.getState().startOperation("push");
      expect(repositoryStore.getState().isOperationRunning("push")).toBe(true);
    });

    it("supports multiple concurrent operations", () => {
      repositoryStore.getState().startOperation("fetch");
      repositoryStore.getState().startOperation("push");
      const ops = repositoryStore.getState().operations;
      expect(ops.has("fetch")).toBe(true);
      expect(ops.has("push")).toBe(true);
      repositoryStore.getState().endOperation("fetch");
      expect(repositoryStore.getState().operations.has("fetch")).toBe(false);
      expect(repositoryStore.getState().operations.has("push")).toBe(true);
    });
  });

  describe("terminal management", () => {
    it("addTerminalGroup creates group, increments counter, sets active", () => {
      repositoryStore.getState().addTerminalGroup();
      const state = repositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(1);
      expect(state.terminalGroups[0].panes).toHaveLength(1);
      expect(state.terminalGroups[0].panes[0]).toEqual({ paneId: 1, name: "Terminal 1" });
      expect(state.terminalGroups[0].activePaneId).toBe(1);
      expect(state.activeGroupId).toBe(1);
      expect(state.terminalIdCounter).toBe(1);
    });

    it("second addTerminalGroup appends", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().addTerminalGroup();
      const state = repositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(2);
      expect(state.terminalGroups[1].groupId).toBe(2);
      expect(state.activeGroupId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("removeTerminalGroup removes and adjusts active", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().removeTerminalGroup(2);
      const state = repositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(1);
      expect(state.activeGroupId).toBe(1);
    });

    it("removeTerminalGroup on last group auto-creates new one", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().removeTerminalGroup(1);
      const state = repositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(2);
      expect(state.terminalGroups[0].panes[0].name).toBe("Terminal 2");
      expect(state.activeGroupId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("splitTerminal adds pane to group and sets active pane", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().splitTerminal(1);
      const state = repositoryStore.getState();
      const group = state.terminalGroups[0];
      expect(group.panes).toHaveLength(2);
      expect(group.panes[1]).toEqual({ paneId: 2, name: "Terminal 2" });
      expect(group.activePaneId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("removePane removes pane and adjusts activePaneId", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().splitTerminal(1);
      repositoryStore.getState().removePane(1, 2);
      const state = repositoryStore.getState();
      const group = state.terminalGroups[0];
      expect(group.panes).toHaveLength(1);
      expect(group.panes[0].paneId).toBe(1);
      expect(group.activePaneId).toBe(1);
    });

    it("removePane on last pane triggers removeTerminalGroup", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().removePane(1, 1);
      const state = repositoryStore.getState();
      // Should auto-create a new group since it was the only group
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(2);
    });

    it("renamePane renames the pane", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().renamePane(1, "My Shell");
      const state = repositoryStore.getState();
      expect(state.terminalGroups[0].panes[0].name).toBe("My Shell");
    });

    it("renamePane does not update when the name is unchanged", () => {
      repositoryStore.getState().addTerminalGroup();
      const before = repositoryStore.getState().terminalGroups;
      repositoryStore.getState().renamePane(1, "Terminal 1");
      expect(repositoryStore.getState().terminalGroups).toBe(before);
    });

    it("setActivePaneInGroup sets active pane", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().splitTerminal(1);
      repositoryStore.getState().setActivePaneInGroup(1, 1);
      expect(repositoryStore.getState().terminalGroups[0].activePaneId).toBe(1);
    });

    it("setActiveGroup sets active group", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().setActiveGroup(1);
      expect(repositoryStore.getState().activeGroupId).toBe(1);
    });

    it("removing first of two groups moves active to remaining", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().setActiveGroup(1);
      repositoryStore.getState().removeTerminalGroup(1);
      const state = repositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.activeGroupId).toBe(2);
    });

    it("split then remove specific pane", () => {
      repositoryStore.getState().addTerminalGroup();
      repositoryStore.getState().splitTerminal(1);
      repositoryStore.getState().splitTerminal(1);
      // Panes: 1, 2, 3 -- active is 3
      repositoryStore.getState().removePane(1, 2);
      const group = repositoryStore.getState().terminalGroups[0];
      expect(group.panes).toHaveLength(2);
      expect(group.panes.map((p) => p.paneId)).toEqual([1, 3]);
      // Active should remain 3 since we removed 2
      expect(group.activePaneId).toBe(3);
    });
  });

  describe("side effects", () => {
    it("setSelectedFile clears blame and cursorLine", () => {
      repositoryStore.setState({
        blame: { path: "test", lines: [] } as never,
        cursorLine: 42,
      });
      repositoryStore.getState().setSelectedFile({
        path: "new-file.ts",
        staged: false,
        groupKind: "workingTree",
      });
      const state = repositoryStore.getState();
      expect(state.selectedFile).toEqual({
        path: "new-file.ts",
        staged: false,
        groupKind: "workingTree",
      });
      expect(state.blame).toBeNull();
      expect(state.cursorLine).toBeNull();
    });

    it("setSelectedFile stores groupKind from the resource group", () => {
      repositoryStore.getState().setSelectedFile({
        path: "conflict.ts",
        staged: false,
        groupKind: "merge",
      });
      expect(repositoryStore.getState().selectedFile).toEqual({
        path: "conflict.ts",
        staged: false,
        groupKind: "merge",
      });
    });

    it("setDiff updates diff", () => {
      const diff = { hunks: [], raw: "diff" } as never;
      repositoryStore.getState().setDiff(diff);
      expect(repositoryStore.getState().diff).toBe(diff);
    });

    it("tags the stored diff with the current selected load", () => {
      const { setSelectedFile, setDiff, bindDiffToCurrentLoad } = repositoryStore.getState();
      setSelectedFile({ path: "conflict.ts", staged: false, groupKind: "merge" });
      expect(repositoryStore.getState().selectedLoadId).toBe(1);

      setDiff({ path: "conflict.ts", original: "", modified: "old", originalLanguage: "ts", fileType: "text" });
      expect(repositoryStore.getState().diffLoadId).toBe(1);

      setSelectedFile({ path: "conflict.ts", staged: false, groupKind: "merge" });
      expect(repositoryStore.getState().selectedLoadId).toBe(2);
      expect(repositoryStore.getState().diffLoadId).toBe(1);

      bindDiffToCurrentLoad();
      expect(repositoryStore.getState().diffLoadId).toBe(2);
    });

    it("setBranches updates branches", () => {
      const branches = [{ name: "main", current: true }] as never[];
      repositoryStore.getState().setBranches(branches);
      expect(repositoryStore.getState().branches).toBe(branches);
    });
  });

  describe("setters", () => {
    it("setAmendMode updates amend mode", () => {
      repositoryStore.getState().setAmendMode(true);
      expect(repositoryStore.getState().amendMode).toBe(true);
      repositoryStore.getState().setAmendMode(false);
      expect(repositoryStore.getState().amendMode).toBe(false);
    });

    it("setFileFilter updates file filter", () => {
      repositoryStore.getState().setFileFilter("*.ts");
      expect(repositoryStore.getState().fileFilter).toBe("*.ts");
    });

    it("setCommitLog updates commit log", () => {
      const log = [{ id: "abc123", message: "test" }] as never[];
      repositoryStore.getState().setCommitLog(log);
      expect(repositoryStore.getState().commitLog).toBe(log);
    });
  });
});
