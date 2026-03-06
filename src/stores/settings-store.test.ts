import { describe, it, expect, beforeEach } from "vitest";
import { useSettingsStore } from "./settings-store";

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left" as const,
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off" as const,
    minimap: false,
    renderWhitespace: "none" as const,
  },
  terminal: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 1.2,
    cursorBlink: true,
    cursorStyle: "block" as const,
  },
  git: { blame: true },
  projects: { projectsDirectory: "", scanDepth: 1 },
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
      expect(settings.projects.scanDepth).toBe(1);
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
        projects: { projectsDirectory: "/home", scanDepth: 3 },
      };
      useSettingsStore.setState({ settings: custom });
      const { settings } = useSettingsStore.getState();
      expect(settings.ui.fontSize).toBe(16);
      expect(settings.editor.tabSize).toBe(2);
      expect(settings.terminal.cursorBlink).toBe(false);
      expect(settings.git.blame).toBe(false);
      expect(settings.projects.projectsDirectory).toBe("/home");
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
      useSettingsStore.getState().updateEditor({ minimap: true });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.editor.minimap).toBe(true);
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
    it("partial update preserves other fields", () => {
      useSettingsStore.getState().updateProjects({ scanDepth: 5 });
      const { projects } = useSettingsStore.getState().settings;
      expect(projects.scanDepth).toBe(5);
      expect(projects.projectsDirectory).toBe("");
    });

    it("saves to localStorage", () => {
      useSettingsStore.getState().updateProjects({ projectsDirectory: "/repos" });
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.projects.projectsDirectory).toBe("/repos");
    });
  });

  describe("resetToDefaults", () => {
    it("resets all sections to defaults", () => {
      useSettingsStore.getState().updateUI({ fontSize: 20 });
      useSettingsStore.getState().updateEditor({ tabSize: 8 });
      useSettingsStore.getState().updateTerminal({ cursorBlink: false });
      useSettingsStore.getState().updateGit({ blame: false });
      useSettingsStore.getState().updateProjects({ scanDepth: 10 });
      useSettingsStore.getState().resetToDefaults();
      const { settings } = useSettingsStore.getState();
      expect(settings.ui.fontSize).toBe(13);
      expect(settings.editor.tabSize).toBe(4);
      expect(settings.terminal.cursorBlink).toBe(true);
      expect(settings.git.blame).toBe(true);
      expect(settings.projects.scanDepth).toBe(1);
    });

    it("saves defaults to localStorage", () => {
      useSettingsStore.getState().updateUI({ fontSize: 20 });
      useSettingsStore.getState().resetToDefaults();
      const stored = JSON.parse(localStorage.getItem(STORAGE_KEY)!);
      expect(stored.ui.fontSize).toBe(13);
    });
  });
});
