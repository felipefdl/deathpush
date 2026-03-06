import { describe, it, expect, beforeEach } from "vitest";
import { useRepositoryStore } from "./repository-store";

beforeEach(() => {
  useRepositoryStore.setState({
    status: null,
    selectedFile: null,
    diff: null,
    branches: [],
    operations: new Set<string>(),
    error: null,
    stashes: [],
    amendMode: false,
    selectedFiles: new Set<string>(),
    fileFilter: "",
    commitLog: [],
    selectedCommit: null,
    commitDetail: null,
    tags: [],
    focusedIndex: null,
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
      const state = useRepositoryStore.getState();
      expect(state.status).toBeNull();
      expect(state.selectedFile).toBeNull();
      expect(state.diff).toBeNull();
      expect(state.branches).toEqual([]);
      expect(state.error).toBeNull();
      expect(state.stashes).toEqual([]);
      expect(state.amendMode).toBe(false);
      expect(state.fileFilter).toBe("");
      expect(state.commitLog).toEqual([]);
      expect(state.selectedCommit).toBeNull();
      expect(state.commitDetail).toBeNull();
      expect(state.tags).toEqual([]);
      expect(state.focusedIndex).toBeNull();
      expect(state.terminalGroups).toEqual([]);
      expect(state.activeGroupId).toBeNull();
      expect(state.terminalIdCounter).toBe(0);
      expect(state.isDiffDirty).toBe(false);
      expect(state.blame).toBeNull();
      expect(state.cursorLine).toBeNull();
    });

    it("has empty Sets for operations and selectedFiles", () => {
      const state = useRepositoryStore.getState();
      expect(state.operations).toBeInstanceOf(Set);
      expect(state.operations.size).toBe(0);
      expect(state.selectedFiles).toBeInstanceOf(Set);
      expect(state.selectedFiles.size).toBe(0);
    });
  });

  describe("toggleFileSelection", () => {
    it("selects a single file without ctrl", () => {
      useRepositoryStore.getState().toggleFileSelection("file-a", false, false);
      expect(useRepositoryStore.getState().selectedFiles).toEqual(new Set(["file-a"]));
    });

    it("adds a file with ctrl+click", () => {
      useRepositoryStore.getState().toggleFileSelection("file-a", false, false);
      useRepositoryStore.getState().toggleFileSelection("file-b", true, false);
      expect(useRepositoryStore.getState().selectedFiles).toEqual(new Set(["file-a", "file-b"]));
    });

    it("toggles off a file with ctrl+click", () => {
      useRepositoryStore.getState().toggleFileSelection("file-a", false, false);
      useRepositoryStore.getState().toggleFileSelection("file-b", true, false);
      useRepositoryStore.getState().toggleFileSelection("file-a", true, false);
      expect(useRepositoryStore.getState().selectedFiles).toEqual(new Set(["file-b"]));
    });

    it("replaces selection without ctrl", () => {
      useRepositoryStore.getState().toggleFileSelection("file-a", false, false);
      useRepositoryStore.getState().toggleFileSelection("file-b", true, false);
      useRepositoryStore.getState().toggleFileSelection("file-c", false, false);
      expect(useRepositoryStore.getState().selectedFiles).toEqual(new Set(["file-c"]));
    });

    it("accumulates multiple ctrl+clicks", () => {
      useRepositoryStore.getState().toggleFileSelection("a", true, false);
      useRepositoryStore.getState().toggleFileSelection("b", true, false);
      useRepositoryStore.getState().toggleFileSelection("c", true, false);
      expect(useRepositoryStore.getState().selectedFiles).toEqual(new Set(["a", "b", "c"]));
    });
  });

  describe("clearFileSelection", () => {
    it("clears all selected files", () => {
      useRepositoryStore.getState().toggleFileSelection("file-a", false, false);
      useRepositoryStore.getState().toggleFileSelection("file-b", true, false);
      useRepositoryStore.getState().clearFileSelection();
      expect(useRepositoryStore.getState().selectedFiles.size).toBe(0);
    });
  });

  describe("operations", () => {
    it("startOperation adds to the set", () => {
      useRepositoryStore.getState().startOperation("fetch");
      expect(useRepositoryStore.getState().operations.has("fetch")).toBe(true);
    });

    it("endOperation removes from the set", () => {
      useRepositoryStore.getState().startOperation("fetch");
      useRepositoryStore.getState().endOperation("fetch");
      expect(useRepositoryStore.getState().operations.has("fetch")).toBe(false);
    });

    it("isOperationRunning returns correct value", () => {
      expect(useRepositoryStore.getState().isOperationRunning("push")).toBe(false);
      useRepositoryStore.getState().startOperation("push");
      expect(useRepositoryStore.getState().isOperationRunning("push")).toBe(true);
    });

    it("supports multiple concurrent operations", () => {
      useRepositoryStore.getState().startOperation("fetch");
      useRepositoryStore.getState().startOperation("push");
      const ops = useRepositoryStore.getState().operations;
      expect(ops.has("fetch")).toBe(true);
      expect(ops.has("push")).toBe(true);
      useRepositoryStore.getState().endOperation("fetch");
      expect(useRepositoryStore.getState().operations.has("fetch")).toBe(false);
      expect(useRepositoryStore.getState().operations.has("push")).toBe(true);
    });
  });

  describe("terminal management", () => {
    it("addTerminalGroup creates group, increments counter, sets active", () => {
      useRepositoryStore.getState().addTerminalGroup();
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(1);
      expect(state.terminalGroups[0].panes).toHaveLength(1);
      expect(state.terminalGroups[0].panes[0]).toEqual({ paneId: 1, name: "Terminal 1" });
      expect(state.terminalGroups[0].activePaneId).toBe(1);
      expect(state.activeGroupId).toBe(1);
      expect(state.terminalIdCounter).toBe(1);
    });

    it("second addTerminalGroup appends", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().addTerminalGroup();
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(2);
      expect(state.terminalGroups[1].groupId).toBe(2);
      expect(state.activeGroupId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("removeTerminalGroup removes and adjusts active", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().removeTerminalGroup(2);
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(1);
      expect(state.activeGroupId).toBe(1);
    });

    it("removeTerminalGroup on last group auto-creates new one", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().removeTerminalGroup(1);
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(2);
      expect(state.terminalGroups[0].panes[0].name).toBe("Terminal 2");
      expect(state.activeGroupId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("splitTerminal adds pane to group and sets active pane", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().splitTerminal(1);
      const state = useRepositoryStore.getState();
      const group = state.terminalGroups[0];
      expect(group.panes).toHaveLength(2);
      expect(group.panes[1]).toEqual({ paneId: 2, name: "Terminal 2" });
      expect(group.activePaneId).toBe(2);
      expect(state.terminalIdCounter).toBe(2);
    });

    it("removePane removes pane and adjusts activePaneId", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().splitTerminal(1);
      useRepositoryStore.getState().removePane(1, 2);
      const state = useRepositoryStore.getState();
      const group = state.terminalGroups[0];
      expect(group.panes).toHaveLength(1);
      expect(group.panes[0].paneId).toBe(1);
      expect(group.activePaneId).toBe(1);
    });

    it("removePane on last pane triggers removeTerminalGroup", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().removePane(1, 1);
      const state = useRepositoryStore.getState();
      // Should auto-create a new group since it was the only group
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.terminalGroups[0].groupId).toBe(2);
    });

    it("renamePane renames the pane", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().renamePane(1, "My Shell");
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups[0].panes[0].name).toBe("My Shell");
    });

    it("setActivePaneInGroup sets active pane", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().splitTerminal(1);
      useRepositoryStore.getState().setActivePaneInGroup(1, 1);
      expect(useRepositoryStore.getState().terminalGroups[0].activePaneId).toBe(1);
    });

    it("setActiveGroup sets active group", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().setActiveGroup(1);
      expect(useRepositoryStore.getState().activeGroupId).toBe(1);
    });

    it("removing first of two groups moves active to remaining", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().setActiveGroup(1);
      useRepositoryStore.getState().removeTerminalGroup(1);
      const state = useRepositoryStore.getState();
      expect(state.terminalGroups).toHaveLength(1);
      expect(state.activeGroupId).toBe(2);
    });

    it("split then remove specific pane", () => {
      useRepositoryStore.getState().addTerminalGroup();
      useRepositoryStore.getState().splitTerminal(1);
      useRepositoryStore.getState().splitTerminal(1);
      // Panes: 1, 2, 3 -- active is 3
      useRepositoryStore.getState().removePane(1, 2);
      const group = useRepositoryStore.getState().terminalGroups[0];
      expect(group.panes).toHaveLength(2);
      expect(group.panes.map((p) => p.paneId)).toEqual([1, 3]);
      // Active should remain 3 since we removed 2
      expect(group.activePaneId).toBe(3);
    });
  });

  describe("side effects", () => {
    it("setSelectedFile clears blame and cursorLine", () => {
      useRepositoryStore.setState({
        blame: { path: "test", lines: [] } as never,
        cursorLine: 42,
      });
      useRepositoryStore.getState().setSelectedFile({ path: "new-file.ts", staged: false });
      const state = useRepositoryStore.getState();
      expect(state.selectedFile).toEqual({ path: "new-file.ts", staged: false });
      expect(state.blame).toBeNull();
      expect(state.cursorLine).toBeNull();
    });

    it("setDiff updates diff", () => {
      const diff = { hunks: [], raw: "diff" } as never;
      useRepositoryStore.getState().setDiff(diff);
      expect(useRepositoryStore.getState().diff).toBe(diff);
    });

    it("setBranches updates branches", () => {
      const branches = [{ name: "main", current: true }] as never[];
      useRepositoryStore.getState().setBranches(branches);
      expect(useRepositoryStore.getState().branches).toBe(branches);
    });
  });

  describe("setters", () => {
    it("setAmendMode updates amend mode", () => {
      useRepositoryStore.getState().setAmendMode(true);
      expect(useRepositoryStore.getState().amendMode).toBe(true);
      useRepositoryStore.getState().setAmendMode(false);
      expect(useRepositoryStore.getState().amendMode).toBe(false);
    });

    it("setFileFilter updates file filter", () => {
      useRepositoryStore.getState().setFileFilter("*.ts");
      expect(useRepositoryStore.getState().fileFilter).toBe("*.ts");
    });

    it("setCommitLog updates commit log", () => {
      const log = [{ id: "abc123", message: "test" }] as never[];
      useRepositoryStore.getState().setCommitLog(log);
      expect(useRepositoryStore.getState().commitLog).toBe(log);
    });
  });
});
