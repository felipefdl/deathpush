import { create } from "zustand";
import type { CommitDetail, CommitEntry, DiffContent, FileBlame, RepositoryStatus, BranchEntry, StashEntry, TagEntry } from "../lib/git-types";

export interface TerminalPane {
  paneId: number;
  name: string;
}

export interface TerminalGroup {
  groupId: number;
  panes: TerminalPane[];
  activePaneId: number;
  splitDirection: "horizontal" | "vertical";
}

interface RepositoryState {
  status: RepositoryStatus | null;
  selectedFile: { path: string; staged: boolean } | null;
  diff: DiffContent | null;
  branches: BranchEntry[];
  operations: Set<string>;
  error: string | null;
  stashes: StashEntry[];
  amendMode: boolean;
  selectedFiles: Set<string>;
  fileFilter: string;
  commitLog: CommitEntry[];
  selectedCommit: string | null;
  commitDetail: CommitDetail | null;
  tags: TagEntry[];
  focusedIndex: number | null;
  terminalGroups: TerminalGroup[];
  activeGroupId: number | null;
  terminalIdCounter: number;
  isDiffDirty: boolean;
  blame: FileBlame | null;
  cursorLine: number | null;

  setTags: (tags: TagEntry[]) => void;
  setStashes: (stashes: StashEntry[]) => void;
  setAmendMode: (amend: boolean) => void;
  setStatus: (status: RepositoryStatus | null) => void;
  setSelectedFile: (file: { path: string; staged: boolean } | null) => void;
  setDiff: (diff: DiffContent | null) => void;
  setBranches: (branches: BranchEntry[]) => void;
  startOperation: (name: string) => void;
  endOperation: (name: string) => void;
  isOperationRunning: (name: string) => boolean;
  setError: (error: string | null) => void;
  toggleFileSelection: (path: string, ctrlKey: boolean, shiftKey: boolean) => void;
  clearFileSelection: () => void;
  setFileFilter: (filter: string) => void;
  setCommitLog: (log: CommitEntry[]) => void;
  setSelectedCommit: (id: string | null) => void;
  setCommitDetail: (detail: CommitDetail | null) => void;
  setFocusedIndex: (index: number | null) => void;
  addTerminalGroup: () => void;
  removeTerminalGroup: (groupId: number) => void;
  setActiveGroup: (groupId: number) => void;
  splitTerminal: (groupId: number) => void;
  splitTerminalVertical: (groupId: number) => void;
  removePane: (groupId: number, paneId: number) => void;
  renamePane: (paneId: number, name: string) => void;
  setActivePaneInGroup: (groupId: number, paneId: number) => void;
  setIsDiffDirty: (dirty: boolean) => void;
  setBlame: (blame: FileBlame | null) => void;
  setCursorLine: (line: number | null) => void;
}

export const useRepositoryStore = create<RepositoryState>((set, get) => ({
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

  setTags: (tags) => set({ tags }),
  setStashes: (stashes) => set({ stashes }),
  setAmendMode: (amend) => set({ amendMode: amend }),
  setStatus: (status) => set({ status }),
  setSelectedFile: (selectedFile) => set({ selectedFile, blame: null, cursorLine: null }),
  setDiff: (diff) => set({ diff }),
  setBranches: (branches) => set({ branches }),
  startOperation: (name) =>
    set((state) => {
      const next = new Set(state.operations);
      next.add(name);
      return { operations: next };
    }),
  endOperation: (name) =>
    set((state) => {
      const next = new Set(state.operations);
      next.delete(name);
      return { operations: next };
    }),
  isOperationRunning: (name) => get().operations.has(name),
  setError: (error) => set({ error }),
  toggleFileSelection: (key, ctrlKey, _shiftKey) => {
    set((state) => {
      if (ctrlKey) {
        const next = new Set(state.selectedFiles);
        if (next.has(key)) {
          next.delete(key);
        } else {
          next.add(key);
        }
        return { selectedFiles: next };
      }
      return { selectedFiles: new Set([key]) };
    });
  },
  clearFileSelection: () => set({ selectedFiles: new Set<string>() }),
  setFileFilter: (filter) => set({ fileFilter: filter }),
  setCommitLog: (log) => set({ commitLog: log }),
  setSelectedCommit: (id) => set({ selectedCommit: id }),
  setCommitDetail: (detail) => set({ commitDetail: detail }),
  setFocusedIndex: (index) => set({ focusedIndex: index }),
  addTerminalGroup: () =>
    set((state) => {
      const num = state.terminalIdCounter + 1;
      const pane: TerminalPane = { paneId: num, name: `Terminal ${num}` };
      const group: TerminalGroup = { groupId: num, panes: [pane], activePaneId: num, splitDirection: "horizontal" };
      return {
        terminalGroups: [...state.terminalGroups, group],
        activeGroupId: num,
        terminalIdCounter: num,
      };
    }),
  removeTerminalGroup: (groupId) =>
    set((state) => {
      const groups = state.terminalGroups.filter((g) => g.groupId !== groupId);
      let active = state.activeGroupId;
      if (active === groupId) {
        const idx = state.terminalGroups.findIndex((g) => g.groupId === groupId);
        const newIdx = Math.min(idx, groups.length - 1);
        active = groups[newIdx]?.groupId ?? null;
      }
      if (groups.length === 0) {
        const num = state.terminalIdCounter + 1;
        const pane: TerminalPane = { paneId: num, name: `Terminal ${num}` };
        const group: TerminalGroup = { groupId: num, panes: [pane], activePaneId: num, splitDirection: "horizontal" };
        return {
          terminalGroups: [group],
          activeGroupId: num,
          terminalIdCounter: num,
        };
      }
      return { terminalGroups: groups, activeGroupId: active };
    }),
  setActiveGroup: (groupId) => set({ activeGroupId: groupId }),
  splitTerminal: (groupId) =>
    set((state) => {
      const num = state.terminalIdCounter + 1;
      const pane: TerminalPane = { paneId: num, name: `Terminal ${num}` };
      return {
        terminalGroups: state.terminalGroups.map((g) =>
          g.groupId === groupId
            ? { ...g, panes: [...g.panes, pane], activePaneId: num, splitDirection: "horizontal" as const }
            : g,
        ),
        terminalIdCounter: num,
      };
    }),
  splitTerminalVertical: (groupId) =>
    set((state) => {
      const num = state.terminalIdCounter + 1;
      const pane: TerminalPane = { paneId: num, name: `Terminal ${num}` };
      return {
        terminalGroups: state.terminalGroups.map((g) =>
          g.groupId === groupId
            ? { ...g, panes: [...g.panes, pane], activePaneId: num, splitDirection: "vertical" as const }
            : g,
        ),
        terminalIdCounter: num,
      };
    }),
  removePane: (groupId, paneId) => {
    const state = get();
    const group = state.terminalGroups.find((g) => g.groupId === groupId);
    if (!group) return;
    if (group.panes.length <= 1) {
      state.removeTerminalGroup(groupId);
      return;
    }
    const panes = group.panes.filter((p) => p.paneId !== paneId);
    const activePaneId = group.activePaneId === paneId ? panes[panes.length - 1].paneId : group.activePaneId;
    set({
      terminalGroups: state.terminalGroups.map((g) =>
        g.groupId === groupId ? { ...g, panes, activePaneId } : g,
      ),
    });
  },
  renamePane: (paneId, name) =>
    set((state) => ({
      terminalGroups: state.terminalGroups.map((g) => ({
        ...g,
        panes: g.panes.map((p) => (p.paneId === paneId ? { ...p, name } : p)),
      })),
    })),
  setActivePaneInGroup: (groupId, paneId) =>
    set((state) => ({
      terminalGroups: state.terminalGroups.map((g) =>
        g.groupId === groupId ? { ...g, activePaneId: paneId } : g,
      ),
    })),
  setIsDiffDirty: (dirty) => set({ isDiffDirty: dirty }),
  setBlame: (blame) => set({ blame }),
  setCursorLine: (line) => set({ cursorLine: line }),
}));
