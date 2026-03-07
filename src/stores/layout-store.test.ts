import { describe, it, expect, beforeEach } from "vitest";
import { useLayoutStore } from "./layout-store";
import { useSettingsStore } from "./settings-store";

const PROJECT_ROOT = "/test/project";
const STORAGE_KEY = `deathpush:layout:${btoa(PROJECT_ROOT)}`;

beforeEach(() => {
  localStorage.clear();
  useLayoutStore.setState({
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
  useSettingsStore.getState().updateUI({ alwaysOpenTerminalOnStart: false });
});

describe("layout store", () => {
  describe("loadForProject", () => {
    it("loads defaults when localStorage is empty", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = useLayoutStore.getState();
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
        }),
      );
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = useLayoutStore.getState();
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
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      const state = useLayoutStore.getState();
      expect(state.sidebarWidth).toBe(300);
      expect(state.mainView).toBe("changes");
    });

    it("normalizes mainView=settings to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "settings" }));
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().mainView).toBe("changes");
    });

    it("normalizes mainView=terminal to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "terminal" }));
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().mainView).toBe("changes");
    });

    it("normalizes mainView=output to changes", () => {
      localStorage.setItem(STORAGE_KEY, JSON.stringify({ mainView: "output" }));
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().mainView).toBe("changes");
    });

    it("opens terminal by default on first project open", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().terminalVisible).toBe(true);
    });

    it("respects saved terminalVisible=false on subsequent opens", () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ terminalVisible: false }),
      );
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().terminalVisible).toBe(false);
    });

    it("alwaysOpenTerminalOnStart overrides saved terminalVisible=false", () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({ terminalVisible: false }),
      );
      useSettingsStore.getState().updateUI({ alwaysOpenTerminalOnStart: true });
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      expect(useLayoutStore.getState().terminalVisible).toBe(true);
    });
  });

  describe("setters persist to localStorage", () => {
    it("setSidebarWidth saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setSidebarWidth(500);
      expect(useLayoutStore.getState().sidebarWidth).toBe(500);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.sidebarWidth).toBe(500);
    });

    it("setTerminalHeight saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setTerminalHeight(400);
      expect(useLayoutStore.getState().terminalHeight).toBe(400);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalHeight).toBe(400);
    });

    it("setDiffMode saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setDiffMode("inline");
      expect(useLayoutStore.getState().diffMode).toBe("inline");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.diffMode).toBe("inline");
    });

    it("setViewMode saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setViewMode("tree");
      expect(useLayoutStore.getState().viewMode).toBe("tree");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.viewMode).toBe("tree");
    });

    it("setPanelTab saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setPanelTab("git-output");
      expect(useLayoutStore.getState().panelTab).toBe("git-output");
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.panelTab).toBe("git-output");
    });

    it("setTerminalVisible saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().setTerminalVisible(true);
      expect(useLayoutStore.getState().terminalVisible).toBe(true);
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalVisible).toBe(true);
    });
  });

  describe("togglePaneCollapsed", () => {
    it("adds an id to collapsedPanes", () => {
      useLayoutStore.getState().togglePaneCollapsed("pane-1");
      expect(useLayoutStore.getState().collapsedPanes).toEqual(["pane-1"]);
    });

    it("removes an existing id from collapsedPanes", () => {
      useLayoutStore.getState().togglePaneCollapsed("pane-1");
      useLayoutStore.getState().togglePaneCollapsed("pane-1");
      expect(useLayoutStore.getState().collapsedPanes).toEqual([]);
    });

    it("handles multiple toggles correctly", () => {
      useLayoutStore.getState().togglePaneCollapsed("pane-1");
      useLayoutStore.getState().togglePaneCollapsed("pane-2");
      expect(useLayoutStore.getState().collapsedPanes).toEqual(["pane-1", "pane-2"]);
      useLayoutStore.getState().togglePaneCollapsed("pane-1");
      expect(useLayoutStore.getState().collapsedPanes).toEqual(["pane-2"]);
    });
  });

  describe("toggleTerminalMaximized", () => {
    it("maximize sets mainView=terminal and terminalMaximized=true", () => {
      useLayoutStore.getState().toggleTerminalMaximized();
      const state = useLayoutStore.getState();
      expect(state.terminalMaximized).toBe(true);
      expect(state.mainView).toBe("terminal");
    });

    it("unmaximize resets mainView to changes", () => {
      useLayoutStore.getState().toggleTerminalMaximized();
      useLayoutStore.getState().toggleTerminalMaximized();
      const state = useLayoutStore.getState();
      expect(state.terminalMaximized).toBe(false);
      expect(state.mainView).toBe("changes");
    });

    it("saves to localStorage", () => {
      useLayoutStore.getState().loadForProject(PROJECT_ROOT);
      useLayoutStore.getState().toggleTerminalMaximized();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminalMaximized).toBe(true);
      expect(stored.mainView).toBe("terminal");
    });
  });
});
