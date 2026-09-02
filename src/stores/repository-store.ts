import { createStore } from "zustand/vanilla";
import type {
  CommitDetail,
  CommitEntry,
  DiffContent,
  FileBlame,
  LastCommitInfo,
  RepositoryIdentity,
  RepositoryStatus,
  ResourceGroupKind,
  BranchEntry,
  SessionActions,
  StashEntry,
  TagEntry,
} from "../lib/git-types";

export type SelectedFile = {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
};

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
  selectedFile: SelectedFile | null;
  selectedLoadId: number;
  diff: DiffContent | null;
  diffLoadId: number | null;
  branches: BranchEntry[];
  operations: Set<string>;
  error: string | null;
  stashes: StashEntry[];
  amendMode: boolean;
  commitMessage: string;
  fileFilter: string;
  commitLog: CommitEntry[];
  selectedCommit: string | null;
  commitDetail: CommitDetail | null;
  fileHistoryPath: string | null;
  tags: TagEntry[];
  lastCommit: LastCommitInfo | null;
  actions: SessionActions | null;
  sessionGeneration: number;
  sessionRevision: number;
  statusGeneration: number;
  statusRevision: number;
  terminalGroups: TerminalGroup[];
  activeGroupId: number | null;
  terminalIdCounter: number;
  isDiffDirty: boolean;
  blame: FileBlame | null;
  cursorLine: number | null;

  setTags: (tags: TagEntry[]) => void;
  setStashes: (stashes: StashEntry[]) => void;
  setAmendMode: (amend: boolean) => void;
  setIdentity: (identity: RepositoryIdentity | null, options?: { reset?: boolean }) => void;
  setSelectedFile: (file: SelectedFile | null) => void;
  setDiff: (diff: DiffContent | null) => void;
  bindDiffToCurrentLoad: () => void;
  setBranches: (branches: BranchEntry[]) => void;
  startOperation: (name: string) => void;
  endOperation: (name: string) => void;
  isOperationRunning: (name: string) => boolean;
  setError: (error: string | null) => void;
  setFileFilter: (filter: string) => void;
  setCommitLog: (log: CommitEntry[]) => void;
  setSelectedCommit: (id: string | null) => void;
  setCommitDetail: (detail: CommitDetail | null) => void;

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

export const repositoryStore = createStore<RepositoryState>((set, get) => ({
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
  commitMessage: "",
  fileFilter: "",

  commitLog: [],
  selectedCommit: null,
  commitDetail: null,
  fileHistoryPath: null,
  tags: [],

  lastCommit: null,
  actions: null,
  sessionGeneration: 0,
  sessionRevision: 0,
  statusGeneration: 0,
  statusRevision: 0,

  terminalGroups: [],
  activeGroupId: null,
  terminalIdCounter: 0,
  isDiffDirty: false,
  blame: null,
  cursorLine: null,

  setTags: (tags) => set({ tags }),
  setStashes: (stashes) => set({ stashes }),
  setAmendMode: (amend) => set({ amendMode: amend }),
  setIdentity: (identity, _options) => {
    if (!identity) {
      set({
        status: null,
        selectedFile: null,
        diff: null,
        diffLoadId: null,
        blame: null,
        cursorLine: null,
        actions: null,
        lastCommit: null,
        commitMessage: "",
        commitDetail: null,
        fileHistoryPath: null,
        sessionGeneration: 0,
        sessionRevision: 0,
        statusGeneration: 0,
        statusRevision: 0,
      });

      return;
    }
    set({
      status: {
        root: identity.root,
        headBranch: identity.headBranch,
        headCommit: null,
        ahead: 0,
        behind: 0,
        groups: [],
        operationState: "none",
      },
    });
  },

  setSelectedFile: (selectedFile) =>
    set((state) => ({
      selectedFile,
      selectedLoadId: state.selectedLoadId + 1,
      blame: null,
      cursorLine: null,
    })),
  setDiff: (diff) =>
    set((state) => ({
      diff,
      diffLoadId: diff ? state.selectedLoadId : null,
    })),
  bindDiffToCurrentLoad: () =>
    set((state) => ({
      diffLoadId: state.diff ? state.selectedLoadId : null,
    })),
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
  setFileFilter: (filter) => set({ fileFilter: filter }),
  setCommitLog: (log) => set({ commitLog: log }),
  setSelectedCommit: (id) => set({ selectedCommit: id }),
  setCommitDetail: (detail) => set({ commitDetail: detail }),
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
            : g
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
            : g
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
      terminalGroups: state.terminalGroups.map((g) => (g.groupId === groupId ? { ...g, panes, activePaneId } : g)),
    });
  },
  renamePane: (paneId, name) => {
    const { terminalGroups } = get();
    if (terminalGroups.some((group) => group.panes.some((pane) => pane.paneId === paneId && pane.name === name))) {
      return;
    }
    set({
      terminalGroups: terminalGroups.map((group) => ({
        ...group,
        panes: group.panes.map((pane) => (pane.paneId === paneId ? { ...pane, name } : pane)),
      })),
    });
  },
  setActivePaneInGroup: (groupId, paneId) =>
    set((state) => ({
      terminalGroups: state.terminalGroups.map((g) => (g.groupId === groupId ? { ...g, activePaneId: paneId } : g)),
    })),
  setIsDiffDirty: (dirty) => set({ isDiffDirty: dirty }),
  setBlame: (blame) => set({ blame }),
  setCursorLine: (line) => set({ cursorLine: line }),
}));
