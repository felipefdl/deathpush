import { describe, it, expect, beforeEach } from "vite-plus/test";
import { layoutStore } from "./layout-store";
import { settingsStore } from "./settings-store";

const PROJECT_ROOT = "/test/project";
const STORAGE_KEY = `deathpush:layout:${btoa(PROJECT_ROOT)}`;

beforeEach(() => {
  localStorage.clear();
  layoutStore.setState({
    sidebarWidth: 300,
    terminalVisible: false,
    terminalHeight: 250,
    mainView: "changes",
    diffMode: "sideBySide",
    viewMode: "list",
    panelTab: "terminal",
    collapsedPanes: [],
    terminalMaximized: false,
  });
  settingsStore.getState().updateUI({ alwaysOpenTerminalOnStart: false });
});

describe("layout store", () => {
  describe("loadForProject", () => {
    it("loads defaults when localStorage is empty", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = layoutStore.getState();
      expect(state.sidebarWidth).toBe(300);
      expect(state.terminalVisible).toBe(true);
      expect(state.terminalHeight).toBe(250);
      expect(state.mainView).toBe("changes");
      expect(state.diffMode).toBe("sideBySide");
      expect(state.viewMode).toBe("list");
      expect(state.panelTab).toBe("terminal");
      expect(state.collapsedPanes).toEqual([]);
      expect(state.terminalMaximized).toBe(false);
    });

    it("loads valid stored layout", () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          sidebarWidth: 400,
          terminalVisible: true,
          terminalHeight: 350,
          mainView: "history",
          diffMode: "inline",
          viewMode: "tree",
          panelTab: "git-output",
          collapsedPanes: ["pane-1"],
          terminalMaximized: false,
        })
      );
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = layoutStore.getState();
      expect(state.sidebarWidth).toBe(400);
      expect(state.terminalVisible).toBe(true);
      expect(state.terminalHeight).toBe(350);
      expect(state.mainView).toBe("history");
      expect(state.diffMode).toBe("inline");
      expect(state.viewMode).toBe("tree");
      expect(state.panelTab).toBe("git-output");
      expect(state.collapsedPanes).toEqual(["pane-1"]);
    });

    it("falls back to defaults on corrupted JSON", () => {
      localStorage.setItem(STORAGE_KEY, "not-valid-json{{{");
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = layoutStore.getState();
      expect(state.sidebarWidth).toBe(300);
      expect(state.mainView).toBe("changes");
    });

    it("normalizes mainView=settings to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "settings" }));
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().mainView).toBe("changes");
    });

    it("normalizes mainView=terminal to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "terminal" }));
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().mainView).toBe("changes");
    });

    it("normalizes mainView=output to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "output" }));
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().mainView).toBe("changes");
    });

    it("opens terminal by default on first project open", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().terminalVisible).toBe(true);
    });

    it("respects saved terminalVisible=false on subsequent opens", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ terminalVisible: false }));
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().terminalVisible).toBe(false);
    });

    it("alwaysOpenTerminalOnStart overrides saved terminalVisible=false", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ terminalVisible: false }));
      settingsStore.getState().updateUI({ alwaysOpenTerminalOnStart: true });
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(layoutStore.getState().terminalVisible).toBe(true);
    });
  });

  describe("setters persist to localStorage", () => {
    it("setSidebarWidth saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setSidebarWidth(500);
      expect(layoutStore.getState().sidebarWidth).toBe(500);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.sidebarWidth).toBe(500);
    });

    it("setTerminalHeight saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setTerminalHeight(400);
      expect(layoutStore.getState().terminalHeight).toBe(400);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalHeight).toBe(400);
    });

    it("setDiffMode saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setDiffMode("inline");
      expect(layoutStore.getState().diffMode).toBe("inline");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.diffMode).toBe("inline");
    });

    it("setViewMode saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setViewMode("tree");
      expect(layoutStore.getState().viewMode).toBe("tree");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.viewMode).toBe("tree");
    });

    it("setPanelTab saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setPanelTab("git-output");
      expect(layoutStore.getState().panelTab).toBe("git-output");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.panelTab).toBe("git-output");
    });

    it("setTerminalVisible saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().setTerminalVisible(true);
      expect(layoutStore.getState().terminalVisible).toBe(true);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalVisible).toBe(true);
    });
  });

  describe("togglePaneCollapsed", () => {
    it("adds an id to collapsedPanes", () => {
      layoutStore.getState().togglePaneCollapsed("pane-1");
      expect(layoutStore.getState().collapsedPanes).toEqual(["pane-1"]);
    });

    it("removes an existing id from collapsedPanes", () => {
      layoutStore.getState().togglePaneCollapsed("pane-1");
      layoutStore.getState().togglePaneCollapsed("pane-1");
      expect(layoutStore.getState().collapsedPanes).toEqual([]);
    });

    it("handles multiple toggles correctly", () => {
      layoutStore.getState().togglePaneCollapsed("pane-1");
      layoutStore.getState().togglePaneCollapsed("pane-2");
      expect(layoutStore.getState().collapsedPanes).toEqual(["pane-1", "pane-2"]);
      layoutStore.getState().togglePaneCollapsed("pane-1");
      expect(layoutStore.getState().collapsedPanes).toEqual(["pane-2"]);
    });
  });

  describe("toggleTerminalMaximized", () => {
    it("maximizing preserves the current main view", () => {
      layoutStore.getState().setMainView("history");
      layoutStore.getState().toggleTerminalMaximized();
      const state = layoutStore.getState();
      expect(state.terminalMaximized).toBe(true);
      expect(state.mainView).toBe("history");
    });

    it("restoring preserves the current main view", () => {
      layoutStore.getState().setMainView("history");
      layoutStore.getState().toggleTerminalMaximized();
      layoutStore.getState().toggleTerminalMaximized();
      const state = layoutStore.getState();
      expect(state.terminalMaximized).toBe(false);
      expect(state.mainView).toBe("history");
    });

    it("saves to localStorage", () => {
      layoutStore.getState().loadForProject(PROJECT_ROOT);
      layoutStore.getState().toggleTerminalMaximized();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalMaximized).toBe(true);
      expect(stored.mainView).toBe("changes");
    });
  });
});
