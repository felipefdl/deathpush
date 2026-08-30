import type { FileTreeBuiltInIconSet, FileTreeDensityKeyword } from "@pierre/trees";
import { createStore } from "zustand/vanilla";
import { normalizeWordWrap } from "../lib/pierre/normalize-editor-settings";
import { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from "../lib/themes/theme-registry";
import { themeStore } from "./theme-store";

export type FontWeight = "normal" | "bold" | "100" | "200" | "300" | "400" | "500" | "600" | "700" | "800" | "900";

export interface EditorSettings {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  tabSize: number;
  wordWrap: "off" | "on";
}

export interface DiffSettings {
  layout: "inline" | "sideBySide";
  showInlineHunkActions: boolean;
  showLineNumbers: boolean;
  diffIndicators: "classic" | "bars" | "none";
  lineDiffType: "word-alt" | "word" | "char" | "none";
  showBackground: boolean;
  hunkSeparators: "simple" | "metadata" | "line-info" | "line-info-basic";
}

export interface TerminalSettings {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  cursorBlink: boolean;
  cursorStyle: "block" | "underline" | "bar";
  scrollback: number;
  copyOnSelect: boolean;
  cursorInactiveStyle: "outline" | "block" | "bar" | "underline" | "none";
  fontWeight: FontWeight;
  fontWeightBold: FontWeight;
  letterSpacing: number;
  cursorWidth: number;
  rightClickSelectsWord: boolean;
  macOptionClickForcesSelection: boolean;
  shellPath: string;
  bellStyle: "off" | "sound" | "visual" | "both";
  colorSaturation: number;
}

export interface GitSettings {
  blame: boolean;
}

export interface UISettings {
  fontFamily: string;
  fontSize: number;
  sidebarPosition: "left" | "right";
  alwaysOpenTerminalOnStart: boolean;
  zoomLevel: number;
  treeDensity: FileTreeDensityKeyword;
  treeIcons: FileTreeBuiltInIconSet;
}

export interface WorkspaceEntry {
  directory: string;
  scanDepth: number;
}

export interface ProjectsSettings {
  workspaces: WorkspaceEntry[];
}

export interface AppSettings {
  ui: UISettings;
  editor: EditorSettings;
  diff: DiffSettings;
  terminal: TerminalSettings;
  git: GitSettings;
  projects: ProjectsSettings;
}

interface SettingsState {
  settings: AppSettings;
  updateUI: (partial: Partial<UISettings>) => void;
  updateEditor: (partial: Partial<EditorSettings>) => void;
  updateDiff: (partial: Partial<DiffSettings>) => void;
  updateTerminal: (partial: Partial<TerminalSettings>) => void;
  updateGit: (partial: Partial<GitSettings>) => void;
  updateProjects: (partial: Partial<ProjectsSettings>) => void;
  zoomIn: () => void;
  zoomOut: () => void;
  resetZoom: () => void;
  resetToDefaults: () => void;
}

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS: AppSettings = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left",
    alwaysOpenTerminalOnStart: false,
    zoomLevel: 0,
    treeDensity: "compact",
    treeIcons: "complete",
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off",
  },
  diff: {
    layout: "sideBySide",
    showInlineHunkActions: false,
    showLineNumbers: true,
    diffIndicators: "none",
    lineDiffType: "word-alt",
    showBackground: true,
    hunkSeparators: "simple",
  },
  terminal: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 1.2,
    cursorBlink: true,
    cursorStyle: "block",
    scrollback: 5000,
    copyOnSelect: false,
    cursorInactiveStyle: "outline",
    fontWeight: "normal",
    fontWeightBold: "bold",
    letterSpacing: 0,
    cursorWidth: 1,
    rightClickSelectsWord: false,
    macOptionClickForcesSelection: false,
    shellPath: "",
    bellStyle: "off",
    colorSaturation: 1.42,
  },
  git: {
    blame: true,
  },
  projects: {
    workspaces: [],
  },
};

const loadSettings = (): AppSettings => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return { ...DEFAULTS };
    const parsed = JSON.parse(raw);
    const editor = { ...DEFAULTS.editor, ...parsed.editor };
    const terminal = { ...DEFAULTS.terminal, ...parsed.terminal };
    return {
      ui: { ...DEFAULTS.ui, ...parsed.ui },
      editor: {
        fontSize: editor.fontSize,
        fontFamily: editor.fontFamily,
        lineHeight: editor.lineHeight,
        tabSize: editor.tabSize,
        wordWrap: normalizeWordWrap(editor.wordWrap),
      },
      diff: { ...DEFAULTS.diff, ...parsed.diff },
      terminal: {
        fontSize: terminal.fontSize,
        fontFamily: terminal.fontFamily,
        lineHeight: terminal.lineHeight,
        cursorBlink: terminal.cursorBlink,
        cursorStyle: terminal.cursorStyle,
        scrollback: terminal.scrollback,
        copyOnSelect: terminal.copyOnSelect,
        cursorInactiveStyle: terminal.cursorInactiveStyle,
        fontWeight: terminal.fontWeight,
        fontWeightBold: terminal.fontWeightBold,
        letterSpacing: terminal.letterSpacing,
        cursorWidth: terminal.cursorWidth,
        rightClickSelectsWord: terminal.rightClickSelectsWord,
        macOptionClickForcesSelection: terminal.macOptionClickForcesSelection,
        shellPath: terminal.shellPath,
        bellStyle: terminal.bellStyle,
        colorSaturation: terminal.colorSaturation,
      },
      git: { ...DEFAULTS.git, ...parsed.git },
      projects: {
        workspaces: Array.isArray(parsed.projects?.workspaces) ? parsed.projects.workspaces : [],
      },
    };
  } catch {
    return { ...DEFAULTS };
  }
};

const saveSettings = (settings: AppSettings) => {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(settings));
};

export const settingsStore = createStore<SettingsState>((set) => ({
  settings: loadSettings(),

  updateUI: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        ui: { ...state.settings.ui, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  updateEditor: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        editor: { ...state.settings.editor, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  updateDiff: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        diff: { ...state.settings.diff, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  updateTerminal: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        terminal: { ...state.settings.terminal, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  updateGit: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        git: { ...state.settings.git, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  updateProjects: (partial) =>
    set((state) => {
      const settings = {
        ...state.settings,
        projects: { ...state.settings.projects, ...partial },
      };
      saveSettings(settings);
      return { settings };
    }),

  zoomIn: () =>
    set((state) => {
      const zoomLevel = Math.min(state.settings.ui.zoomLevel + 1, 9);
      const settings = { ...state.settings, ui: { ...state.settings.ui, zoomLevel } };
      saveSettings(settings);
      return { settings };
    }),

  zoomOut: () =>
    set((state) => {
      const zoomLevel = Math.max(state.settings.ui.zoomLevel - 1, -5);
      const settings = { ...state.settings, ui: { ...state.settings.ui, zoomLevel } };
      saveSettings(settings);
      return { settings };
    }),

  resetZoom: () =>
    set((state) => {
      const settings = { ...state.settings, ui: { ...state.settings.ui, zoomLevel: 0 } };
      saveSettings(settings);
      return { settings };
    }),

  resetToDefaults: () => {
    const settings = { ...DEFAULTS };
    saveSettings(settings);
    set({ settings });

    localStorage.removeItem("deathpush:theme");
    localStorage.removeItem("deathpush:preferred-dark-theme");
    localStorage.removeItem("deathpush:preferred-light-theme");

    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    const id = prefersDark ? DEFAULT_DARK_THEME_ID : DEFAULT_LIGHT_THEME_ID;
    void themeStore.getState().setTheme(id);
  },
}));
