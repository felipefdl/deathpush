import { describe, it, expect, beforeEach, vi } from "vite-plus/test";

vi.mock("../lib/pierre/worker", () => ({
  getPierreWorkerPool: vi.fn(),
  applyPierrePoolTheme: vi.fn(),
}));

import { settingsStore } from "./settings-store";

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left" as const,
    alwaysOpenTerminalOnStart: false,
    zoomLevel: 0,
    treeDensity: "compact" as const,
    treeIcons: "complete" as const,
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off" as const,
  },
  diff: {
    layout: "sideBySide" as const,
    showInlineHunkActions: false,
    showLineNumbers: true,
    diffIndicators: "none" as const,
    lineDiffType: "word-alt" as const,
    showBackground: true,
    hunkSeparators: "simple" as const,
  },
  terminal: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 1.2,
    cursorBlink: true,
    cursorStyle: "block" as const,
    scrollback: 5000,
    copyOnSelect: false,
    cursorInactiveStyle: "outline" as const,
    fontWeight: "normal" as const,
    fontWeightBold: "bold" as const,
    letterSpacing: 0,
    cursorWidth: 1,
    rightClickSelectsWord: false,
    macOptionClickForcesSelection: false,
    shellPath: "",
    bellStyle: "off" as const,
    colorSaturation: 1.42,
  },
  git: { blame: true },
  projects: { workspaces: [] },
};

beforeEach(() => {
  localStorage.clear();
  settingsStore.setState({
    settings: structuredClone(DEFAULTS),
  });
});

describe("settings store", () => {
  describe("loadSettings / resetToDefaults", () => {
    it("resetToDefaults restores all defaults", () => {
      settingsStore.getState().updateUI({ fontSize: 20 });
      settingsStore.getState().resetToDefaults();
      const { settings } = settingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
      expect(settings.editor.tabSize).toBe(4);
      expect(settings.ui.treeDensity).toBe("compact");
      expect(settings.ui.treeIcons).toBe("complete");
      expect(settings.diff).toEqual(DEFAULTS.diff);
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.workspaces).toEqual([]);
    });

    it("resetToDefaults saves defaults to localStorage", () => {
      settingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
      expect(stored.editor.tabSize).toBe(4);
      expect(stored.diff).toEqual(DEFAULTS.diff);
    });

    it("handles corrupted localStorage gracefully via resetToDefaults", () => {
      localStorage.setItem(STORAGE_KEY, "broken{json");
      settingsStore.getState().resetToDefaults();
      const { settings } = settingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
    });

    it("preserves full stored settings when all sections present", () => {
      const custom = {
        ui: { ...DEFAULTS.ui, fontSize: 16 },
        editor: { ...DEFAULTS.editor, tabSize: 2 },
        diff: { ...DEFAULTS.diff, diffIndicators: "classic" as const },
        terminal: { ...DEFAULTS.terminal, cursorBlink: false },
        git: { blame: false },
        projects: { workspaces: [{ directory: "/home", scanDepth: 3 }] },
      };
      settingsStore.setState({ settings: custom });
      const { settings } = settingsStore.getState();
      expect(settings.ui.fontSize).toBe(16);
      expect(settings.editor.tabSize).toBe(2);
      expect(settings.diff.diffIndicators).toBe("classic");
      expect(settings.terminal.cursorBlink).toBe(false);
      expect(settings.git.blame).toBe(false);
      expect(settings.projects.workspaces).toEqual([{ directory: "/home", scanDepth: 3 }]);
    });

    it("omits renderWhitespace from editor defaults", () => {
      settingsStore.getState().resetToDefaults();
      expect(settingsStore.getState().settings.editor).not.toHaveProperty("renderWhitespace");
    });

    it("normalizes legacy wordWrap and drops renderWhitespace on load", async () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          editor: {
            ...DEFAULTS.editor,
            wordWrap: "wordWrapColumn",
            renderWhitespace: "all",
          },
        })
      );
      vi.resetModules();
      // Reloading is the behavior under test; a static import cannot re-read localStorage.
      const { settingsStore: reloaded } = await import("./settings-store");
      const { editor } = reloaded.getState().settings;
      expect(editor.wordWrap).toBe("on");
      expect(editor).not.toHaveProperty("renderWhitespace");
    });
  });

  describe("updateUI", () => {
    it("partial update preserves other fields", () => {
      settingsStore.getState().updateUI({ fontSize: 18 });
      const { ui } = settingsStore.getState().settings;
      expect(ui.fontSize).toBe(18);
      expect(ui.sidebarPosition).toBe("left");
      expect(ui.fontFamily).toBe(DEFAULTS.ui.fontFamily);
    });

    it("saves to localStorage", () => {
      settingsStore.getState().updateUI({ fontSize: 18 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(18);
    });

    it("multiple updates accumulate", () => {
      settingsStore.getState().updateUI({ fontSize: 18 });
      settingsStore.getState().updateUI({ sidebarPosition: "right" });
      const { ui } = settingsStore.getState().settings;
      expect(ui.fontSize).toBe(18);
      expect(ui.sidebarPosition).toBe("right");
    });
  });

  describe("updateEditor", () => {
    it("partial update preserves other fields", () => {
      settingsStore.getState().updateEditor({ tabSize: 2 });
      const { editor } = settingsStore.getState().settings;
      expect(editor.tabSize).toBe(2);
      expect(editor.fontSize).toBe(13);
      expect(editor.wordWrap).toBe("off");
    });

    it("saves to localStorage", () => {
      settingsStore.getState().updateEditor({ tabSize: 2 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.editor.tabSize).toBe(2);
    });
  });

  describe("updateTerminal", () => {
    it("partial update preserves other fields", () => {
      settingsStore.getState().updateTerminal({ cursorStyle: "bar" });
      const { terminal } = settingsStore.getState().settings;
      expect(terminal.cursorStyle).toBe("bar");
      expect(terminal.fontSize).toBe(13);
      expect(terminal.cursorBlink).toBe(true);
    });

    it("saves to localStorage", () => {
      settingsStore.getState().updateTerminal({ fontSize: 16 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminal.fontSize).toBe(16);
    });

    it("supported terminal settings update independently", () => {
      settingsStore
        .getState()
        .updateTerminal({ rightClickSelectsWord: true, macOptionClickForcesSelection: true, bellStyle: "sound" });
      const { terminal } = settingsStore.getState().settings;
      expect(terminal.rightClickSelectsWord).toBe(true);
      expect(terminal.macOptionClickForcesSelection).toBe(true);
      expect(terminal.bellStyle).toBe("sound");
      expect(terminal.cursorBlink).toBe(true);
    });

    it("drops unsupported terminal settings on load", async () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify({
          terminal: {
            ...DEFAULTS.terminal,
            scrollSensitivity: 2,
            wordSeparator: " ",
          },
        })
      );
      vi.resetModules();
      // Reloading is the behavior under test; a static import cannot re-read localStorage.
      const { settingsStore: reloaded } = await import("./settings-store");
      const { terminal } = reloaded.getState().settings;
      expect(terminal).not.toHaveProperty("scrollSensitivity");
      expect(terminal).not.toHaveProperty("wordSeparator");
    });
  });

  describe("updateGit", () => {
    it("partial update works", () => {
      settingsStore.getState().updateGit({ blame: false });
      expect(settingsStore.getState().settings.git.blame).toBe(false);
    });

    it("saves to localStorage", () => {
      settingsStore.getState().updateGit({ blame: false });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.git.blame).toBe(false);
    });
  });

  describe("updateProjects", () => {
    it("updates workspaces array", () => {
      const workspaces = [{ directory: "/repos", scanDepth: 2 }];
      settingsStore.getState().updateProjects({ workspaces });
      const { projects } = settingsStore.getState().settings;
      expect(projects.workspaces).toEqual(workspaces);
    });

    it("saves to localStorage", () => {
      const workspaces = [{ directory: "/repos", scanDepth: 1 }];
      settingsStore.getState().updateProjects({ workspaces });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.projects.workspaces).toEqual(workspaces);
    });
  });

  describe("resetToDefaults", () => {
    it("resets all sections to defaults", () => {
      settingsStore.getState().updateUI({ fontSize: 20 });
      settingsStore.getState().updateEditor({ tabSize: 8 });
      settingsStore.getState().updateTerminal({ cursorBlink: false });
      settingsStore.getState().updateGit({ blame: false });
      settingsStore.getState().updateProjects({ workspaces: [{ directory: "/tmp", scanDepth: 10 }] });
      settingsStore.getState().resetToDefaults();
      const { settings } = settingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
      expect(settings.editor.tabSize).toBe(4);
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.workspaces).toEqual([]);
    });

    it("saves defaults to localStorage", () => {
      settingsStore.getState().updateUI({ fontSize: 20 });
      settingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
    });
  });

  describe("zoom", () => {
    it("zoomIn increments zoomLevel", () => {
      settingsStore.getState().zoomIn();
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(1);
    });

    it("zoomOut decrements zoomLevel", () => {
      settingsStore.getState().zoomOut();
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(-1);
    });

    it("resetZoom sets zoomLevel to 0", () => {
      settingsStore.getState().zoomIn();
      settingsStore.getState().zoomIn();
      settingsStore.getState().resetZoom();
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(0);
    });

    it("clamps zoomLevel at max 9", () => {
      for (let i = 0; i < 15; i++) {
        settingsStore.getState().zoomIn();
      }
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(9);
    });

    it("clamps zoomLevel at min -5", () => {
      for (let i = 0; i < 10; i++) {
        settingsStore.getState().zoomOut();
      }
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(-5);
    });

    it("persists zoomLevel to localStorage", () => {
      settingsStore.getState().zoomIn();
      settingsStore.getState().zoomIn();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.zoomLevel).toBe(2);
    });

    it("resetToDefaults resets zoomLevel", () => {
      settingsStore.getState().zoomIn();
      settingsStore.getState().zoomIn();
      settingsStore.getState().resetToDefaults();
      expect(settingsStore.getState().settings.ui.zoomLevel).toBe(0);
    });
  });
});
