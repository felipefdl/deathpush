import { create } from "zustand";

export type MainView = "changes" | "history" | "settings" | "terminal" | "output";

interface LayoutState {
  sidebarWidth: number;
  terminalVisible: boolean;
  terminalHeight: number;
  mainView: MainView;
  diffMode: "inline" | "sideBySide";
  viewMode: "list" | "tree";
  panelTab: "terminal" | "git-output";
  collapsedPanes: string[];
  terminalMaximized: boolean;

  setSidebarWidth: (width: number) => void;
  setTerminalVisible: (visible: boolean) => void;
  setTerminalHeight: (height: number) => void;
  setMainView: (view: MainView) => void;
  setDiffMode: (mode: "inline" | "sideBySide") => void;
  setViewMode: (mode: "list" | "tree") => void;
  setPanelTab: (tab: "terminal" | "git-output") => void;
  togglePaneCollapsed: (id: string) => void;
  setTerminalMaximized: (maximized: boolean) => void;
  toggleTerminalMaximized: () => void;
  loadForProject: (root: string) => void;
}

interface PersistedLayout {
  sidebarWidth: number;
  terminalVisible: boolean;
  terminalHeight: number;
  mainView: MainView;
  diffMode: "inline" | "sideBySide";
  viewMode: "list" | "tree";
  panelTab: "terminal" | "git-output";
  collapsedPanes: string[];
  terminalMaximized: boolean;
}

const DEFAULTS: PersistedLayout = {
  sidebarWidth: 300,
  terminalVisible: false,
  terminalHeight: 250,
  mainView: "changes",
  diffMode: "sideBySide",
  viewMode: "list",
  panelTab: "terminal",
  collapsedPanes: [],
  terminalMaximized: false,
};

let currentProjectRoot: string | null = null;

const storageKey = (root: string) => `deathpush:layout:${btoa(root)}`;

const loadLayout = (root: string): PersistedLayout => {
  try {
    const raw = localStorage.getItem(storageKey(root));
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw);
    const layout: PersistedLayout = { ...DEFAULTS, ...parsed };
    if (layout.mainView === "settings" || layout.mainView === "terminal" || layout.mainView === "output") {
      layout.mainView = "changes";
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
    diffMode: state.diffMode,
    viewMode: state.viewMode,
    panelTab: state.panelTab,
    collapsedPanes: state.collapsedPanes,
    terminalMaximized: state.terminalMaximized,
  };
  localStorage.setItem(storageKey(currentProjectRoot), JSON.stringify(data));
};

export const useLayoutStore = create<LayoutState>((set, get) => ({
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
    set({ mainView });
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
    const state = get();
    const next = !state.terminalMaximized;
    if (next) {
      set({ terminalMaximized: true, mainView: "terminal" });
    } else {
      set({ terminalMaximized: false, mainView: "changes" });
    }
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
