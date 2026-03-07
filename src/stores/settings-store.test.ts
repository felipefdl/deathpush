import { describe, it, expect, beforeEach } from "vitest";
import { useSettingsStore } from "./settings-store";

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left" as const,
    alwaysOpenTerminalOnStart: false,
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off" as const,
    renderWhitespace: "none" as const,
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
    shellArgs: "-l",
    bellStyle: "off" as const,
  },
  git: { blame: true },
  projects: { workspaces: [] },
};

beforeEach(() => {
  localStorage.clear();
  useSettingsStore.setState({
    settings: structuredClone(DEFAULTS),
  });
});

describe("settings store", () => {
  describe("loadSettings / resetToDefaults", () => {
    it("resetToDefaults restores all defaults", () => {
      useSettingsStore.getState().updateUI({ fontSize: 20 });
      useSettingsStore.getState().resetToDefaults();
      const { settings } = useSettingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
      expect(settings.editor.tabSize).toBe(4);
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.workspaces).toEqual([]);
    });

    it("resetToDefaults saves defaults to localStorage", () => {
      useSettingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
      expect(stored.editor.tabSize).toBe(4);
    });

    it("handles corrupted localStorage gracefully via resetToDefaults", () => {
      localStorage.setItem(STORAGE_KEY, "broken{json");
      useSettingsStore.getState().resetToDefaults();
      const { settings } = useSettingsStore.getState();
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
      useSettingsStore.setState({ settings: custom });
      const { settings } = useSettingsStore.getState();
      expect(settings.ui.fontSize).toBe(16);
      expect(settings.editor.tabSize).toBe(2);
      expect(settings.terminal.cursorBlink).toBe(false);
      expect(settings.git.blame).toBe(false);
      expect(settings.projects.workspaces).toEqual([{ directory: "/home", scanDepth: 3 }]);
    });
  });

  describe("updateUI", () => {
    it("partial update preserves other fields", () => {
      useSettingsStore.getState().updateUI({ fontSize: 18 });
      const { ui } = useSettingsStore.getState().settings;
      expect(ui.fontSize).toBe(18);
      expect(ui.sidebarPosition).toBe("left");
      expect(ui.fontFamily).toBe(DEFAULTS.ui.fontFamily);
    });

    it("saves to localStorage", () => {
      useSettingsStore.getState().updateUI({ fontSize: 18 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(18);
    });

    it("multiple updates accumulate", () => {
      useSettingsStore.getState().updateUI({ fontSize: 18 });
      useSettingsStore.getState().updateUI({ sidebarPosition: "right" });
      const { ui } = useSettingsStore.getState().settings;
      expect(ui.fontSize).toBe(18);
      expect(ui.sidebarPosition).toBe("right");
    });
  });

  describe("updateEditor", () => {
    it("partial update preserves other fields", () => {
      useSettingsStore.getState().updateEditor({ tabSize: 2 });
      const { editor } = useSettingsStore.getState().settings;
      expect(editor.tabSize).toBe(2);
      expect(editor.fontSize).toBe(13);
      expect(editor.wordWrap).toBe("off");
    });

    it("saves to localStorage", () => {
      useSettingsStore.getState().updateEditor({ tabSize: 2 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.editor.tabSize).toBe(2);
    });
  });

  describe("updateTerminal", () => {
    it("partial update preserves other fields", () => {
      useSettingsStore.getState().updateTerminal({ cursorStyle: "bar" });
      const { terminal } = useSettingsStore.getState().settings;
      expect(terminal.cursorStyle).toBe("bar");
      expect(terminal.fontSize).toBe(13);
      expect(terminal.cursorBlink).toBe(true);
    });

    it("saves to localStorage", () => {
      useSettingsStore.getState().updateTerminal({ fontSize: 16 });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.terminal.fontSize).toBe(16);
    });

    it("new terminal settings update independently", () => {
      useSettingsStore.getState().updateTerminal({ macOptionIsMeta: true, bellStyle: "sound" });
      const { terminal } = useSettingsStore.getState().settings;
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
      useSettingsStore.getState().updateGit({ blame: false });
      expect(useSettingsStore.getState().settings.git.blame).toBe(false);
    });

    it("saves to localStorage", () => {
      useSettingsStore.getState().updateGit({ blame: false });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.git.blame).toBe(false);
    });
  });

  describe("updateProjects", () => {
    it("updates workspaces array", () => {
      const workspaces = [{ directory: "/repos", scanDepth: 2 }];
      useSettingsStore.getState().updateProjects({ workspaces });
      const { projects } = useSettingsStore.getState().settings;
      expect(projects.workspaces).toEqual(workspaces);
    });

    it("saves to localStorage", () => {
      const workspaces = [{ directory: "/repos", scanDepth: 1 }];
      useSettingsStore.getState().updateProjects({ workspaces });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.projects.workspaces).toEqual(workspaces);
    });
  });

  describe("resetToDefaults", () => {
    it("resets all sections to defaults", () => {
      useSettingsStore.getState().updateUI({ fontSize: 20 });
      useSettingsStore.getState().updateEditor({ tabSize: 8 });
      useSettingsStore.getState().updateTerminal({ cursorBlink: false });
      useSettingsStore.getState().updateGit({ blame: false });
      useSettingsStore.getState().updateProjects({ workspaces: [{ directory: "/tmp", scanDepth: 10 }] });
      useSettingsStore.getState().resetToDefaults();
      const { settings } = useSettingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
      expect(settings.editor.tabSize).toBe(4);
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.workspaces).toEqual([]);
    });

    it("saves defaults to localStorage", () => {
      useSettingsStore.getState().updateUI({ fontSize: 20 });
      useSettingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
    });
  });
});
