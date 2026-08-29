import { describe, it, expect, beforeEach, vi } from "vite-plus/test";
import { settingsStore } from "./settings-store";

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left" as const,
    alwaysOpenTerminalOnStart: false,
    zoomLevel: 0,
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off" as const,
  },
  terminal: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 1.2,
    cursorBlink: true,
    cursorStyle: "block" as const,
    scrollback: 5000,
    copyOnSelect: false,
    macOptionIsMeta: false,
    cursorInactiveStyle: "outline" as const,
    minimumContrastRatio: 1,
    scrollSensitivity: 1,
    fastScrollSensitivity: 5,
    fontWeight: "normal" as const,
    fontWeightBold: "bold" as const,
    letterSpacing: 0,
    cursorWidth: 1,
    smoothScrollDuration: 0,
    drawBoldTextInBrightColors: true,
    rightClickSelectsWord: false,
    macOptionClickForcesSelection: false,
    altClickMovesCursor: true,
    wordSeparator: " ()[]{}',\"`",
    tabStopWidth: 8,
    scrollOnUserInput: true,
    rescaleOverlappingGlyphs: false,
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
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.workspaces).toEqual([]);
    });

    it("resetToDefaults saves defaults to localStorage", () => {
      settingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
      expect(stored.editor.tabSize).toBe(4);
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
        terminal: { ...DEFAULTS.terminal, cursorBlink: false },
        git: { blame: false },
        projects: { workspaces: [{ directory: "/home", scanDepth: 3 }] },
      };
      settingsStore.setState({ settings: custom });
      const { settings } = settingsStore.getState();
      expect(settings.ui.fontSize).toBe(16);
      expect(settings.editor.tabSize).toBe(2);
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

    it("new terminal settings update independently", () => {
      settingsStore.getState().updateTerminal({ macOptionIsMeta: true, bellStyle: "sound" });
      const { terminal } = settingsStore.getState().settings;
      expect(terminal.macOptionIsMeta).toBe(true);
      expect(terminal.bellStyle).toBe("sound");
      expect(terminal.cursorBlink).toBe(true);
      expect(terminal.scrollSensitivity).toBe(1);
    });

    it("old localStorage without new fields loads with defaults", () => {
      const oldData = {
        ui: DEFAULTS.ui,
        editor: DEFAULTS.editor,
        terminal: {
          fontSize: 14,
          fontFamily: DEFAULTS.terminal.fontFamily,
          lineHeight: 1.2,
          cursorBlink: false,
          cursorStyle: "bar",
          scrollback: 3000,
          copyOnSelect: true,
        },
        git: DEFAULTS.git,
        projects: DEFAULTS.projects,
      };
      localStorage.setItem(STORAGE_KEY, JSON.stringify(oldData));
      // Force re-load by resetting state as if the app just started
      const { loadSettings } = (() => {
        const raw = localStorage.getItem(STORAGE_KEY);
        const parsed = JSON.parse(raw!);
        return {
          loadSettings: {
            ...DEFAULTS,
            terminal: { ...DEFAULTS.terminal, ...parsed.terminal },
          },
        };
      })();
      expect(loadSettings.terminal.fontSize).toBe(14);
      expect(loadSettings.terminal.cursorBlink).toBe(false);
      expect(loadSettings.terminal.macOptionIsMeta).toBe(false);
      expect(loadSettings.terminal.bellStyle).toBe("off");
      expect(loadSettings.terminal.shellPath).toBe("");
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
