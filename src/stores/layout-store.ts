import { createStore } from "zustand/vanilla";
import { settingsStore } from "./settings-store";

export type MainView = "changes" | "history" | "settings" | "file";
export type SidebarView = "scm" | "explorer";

interface LayoutState {
  sidebarWidth: number;
  terminalVisible: boolean;
  terminalHeight: number;
  mainView: MainView;
  sidebarView: SidebarView;
  diffMode: "inline" | "sideBySide";
  viewMode: "list" | "tree";
  panelTab: "terminal" | "git-output";
  collapsedPanes: string[];
  terminalMaximized: boolean;
  historyListWidth: number;

  setSidebarWidth: (width: number) => void;
  setTerminalVisible: (visible: boolean) => void;
  setTerminalHeight: (height: number) => void;
  setMainView: (view: MainView) => void;
  setSidebarView: (view: SidebarView) => void;
  setDiffMode: (mode: "inline" | "sideBySide") => void;
  setViewMode: (mode: "list" | "tree") => void;
  setPanelTab: (tab: "terminal" | "git-output") => void;
  togglePaneCollapsed: (id: string) => void;
  setTerminalMaximized: (maximized: boolean) => void;
  dockTerminal: () => void;
  toggleTerminalMaximized: () => void;
  setHistoryListWidth: (width: number) => void;
  loadForProject: (root: string) => void;
}

interface PersistedLayout {
  sidebarWidth: number;
  terminalVisible: boolean;
  terminalHeight: number;
  mainView: MainView;
  sidebarView: SidebarView;
  diffMode: "inline" | "sideBySide";
  viewMode: "list" | "tree";
  panelTab: "terminal" | "git-output";
  collapsedPanes: string[];
  terminalMaximized: boolean;
  historyListWidth: number;
}

const DEFAULTS: PersistedLayout = {
  sidebarWidth: 300,
  terminalVisible: true,
  terminalHeight: 250,
  mainView: "changes",
  sidebarView: "scm",
  diffMode: "sideBySide",
  viewMode: "list",
  panelTab: "terminal",
  collapsedPanes: [],
  terminalMaximized: false,
  historyListWidth: 300,
};

let currentProjectRoot: string | null = null;

const storageKey = (root: string) => `deathpush:layout:${btoa(root)}`;

const loadLayout = (root: string): PersistedLayout => {
  try {
    const raw = localStorage.getItem(storageKey(root));
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw);
    const layout: PersistedLayout = { ...DEFAULTS, ...parsed };
    if (layout.mainView !== "changes" && layout.mainView !== "history") {
      layout.mainView = "changes";
    }
    const { alwaysOpenTerminalOnStart } = settingsStore.getState().settings.ui;
    if (alwaysOpenTerminalOnStart) {
      layout.terminalVisible = true;
    }
    return layout;
  } catch {
    return { ...DEFAULTS };
  }
};

const saveLayout = (state: LayoutState) => {
  if (!currentProjectRoot) return;
  const data: PersistedLayout = {
    sidebarWidth: state.sidebarWidth,
    terminalVisible: state.terminalVisible,
    terminalHeight: state.terminalHeight,
    mainView: state.mainView,
    sidebarView: state.sidebarView,
    diffMode: state.diffMode,
    viewMode: state.viewMode,
    panelTab: state.panelTab,
    collapsedPanes: state.collapsedPanes,
    terminalMaximized: state.terminalMaximized,
    historyListWidth: state.historyListWidth,
  };
  localStorage.setItem(storageKey(currentProjectRoot), JSON.stringify(data));
};

export const layoutStore = createStore<LayoutState>((set, get) => ({
  ...DEFAULTS,

  setSidebarWidth: (sidebarWidth) => {
    set({ sidebarWidth });
    saveLayout(get());
  },
  setTerminalVisible: (terminalVisible) => {
    set({ terminalVisible });
    saveLayout(get());
  },
  setTerminalHeight: (terminalHeight) => {
    set({ terminalHeight });
    saveLayout(get());
  },
  setMainView: (mainView) => {
    const dock = mainView !== "changes" && mainView !== "file";
    set(dock ? { mainView, terminalMaximized: false } : { mainView });
    saveLayout(get());
  },
  setSidebarView: (sidebarView) => {
    const { mainView } = get();
    const update: Partial<LayoutState> = { sidebarView };
    if (mainView === "changes" || mainView === "file") {
      update.mainView = sidebarView === "explorer" ? "file" : "changes";
    }
    set(update);
    saveLayout(get());
  },
  setDiffMode: (diffMode) => {
    set({ diffMode });
    saveLayout(get());
  },
  setViewMode: (viewMode) => {
    set({ viewMode });
    saveLayout(get());
  },
  setPanelTab: (panelTab) => {
    set({ panelTab });
    saveLayout(get());
  },
  setTerminalMaximized: (terminalMaximized) => {
    set({ terminalMaximized });
    saveLayout(get());
  },
  toggleTerminalMaximized: () => {
    set((state) => ({ terminalMaximized: !state.terminalMaximized }));
    saveLayout(get());
  },
  dockTerminal: () => {
    set({ terminalMaximized: false });
    saveLayout(get());
  },
  setHistoryListWidth: (historyListWidth) => {
    set({ historyListWidth });
    saveLayout(get());
  },
  togglePaneCollapsed: (id) => {
    set((state) => {
      const collapsed = state.collapsedPanes.includes(id)
        ? state.collapsedPanes.filter((p) => p !== id)
        : [...state.collapsedPanes, id];
      return { collapsedPanes: collapsed };
    });
    saveLayout(get());
  },
  loadForProject: (root) => {
    currentProjectRoot = root;
    const layout = loadLayout(root);
    set(layout);
  },
}));
