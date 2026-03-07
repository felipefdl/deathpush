import { create } from "zustand";

export interface EditorSettings {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  tabSize: number;
  wordWrap: "off" | "on" | "wordWrapColumn" | "bounded";
  renderWhitespace: "none" | "boundary" | "selection" | "trailing" | "all";
}

export interface TerminalSettings {
  fontSize: number;
  fontFamily: string;
  lineHeight: number;
  cursorBlink: boolean;
  cursorStyle: "block" | "underline" | "bar";
  scrollback: number;
  copyOnSelect: boolean;
  // Tier 1
  macOptionIsMeta: boolean;
  cursorInactiveStyle: "outline" | "block" | "bar" | "underline" | "none";
  minimumContrastRatio: number;
  scrollSensitivity: number;
  fastScrollSensitivity: number;
  fontWeight: string;
  fontWeightBold: string;
  // Tier 2
  letterSpacing: number;
  cursorWidth: number;
  smoothScrollDuration: number;
  drawBoldTextInBrightColors: boolean;
  rightClickSelectsWord: boolean;
  macOptionClickForcesSelection: boolean;
  altClickMovesCursor: boolean;
  // Tier 3
  wordSeparator: string;
  tabStopWidth: number;
  scrollOnUserInput: boolean;
  rescaleOverlappingGlyphs: boolean;
  // Non-xterm
  shellPath: string;
  shellArgs: string;
  bellStyle: "off" | "sound" | "visual" | "both";
}

export interface GitSettings {
  blame: boolean;
}

export interface UISettings {
  fontFamily: string;
  fontSize: number;
  sidebarPosition: "left" | "right";
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
  terminal: TerminalSettings;
  git: GitSettings;
  projects: ProjectsSettings;
}

interface SettingsState {
  settings: AppSettings;
  updateUI: (partial: Partial<UISettings>) => void;
  updateEditor: (partial: Partial<EditorSettings>) => void;
  updateTerminal: (partial: Partial<TerminalSettings>) => void;
  updateGit: (partial: Partial<GitSettings>) => void;
  updateProjects: (partial: Partial<ProjectsSettings>) => void;
  resetToDefaults: () => void;
}

const STORAGE_KEY = "deathpush:settings";

const DEFAULTS: AppSettings = {
  ui: {
    fontFamily: "-apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif",
    fontSize: 13,
    sidebarPosition: "left",
  },
  editor: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 20,
    tabSize: 4,
    wordWrap: "off",
    renderWhitespace: "none",
  },
  terminal: {
    fontSize: 13,
    fontFamily: "'MesloLGS Nerd Font Mono', 'Menlo', 'Monaco', 'Courier New', monospace",
    lineHeight: 1.2,
    cursorBlink: true,
    cursorStyle: "block",
    scrollback: 5000,
    copyOnSelect: false,
    macOptionIsMeta: false,
    cursorInactiveStyle: "outline",
    minimumContrastRatio: 1,
    scrollSensitivity: 1,
    fastScrollSensitivity: 5,
    fontWeight: "normal",
    fontWeightBold: "bold",
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
    bellStyle: "off",
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
    return {
      ui: { ...DEFAULTS.ui, ...parsed.ui },
      editor: { ...DEFAULTS.editor, ...parsed.editor },
      terminal: { ...DEFAULTS.terminal, ...parsed.terminal },
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

export const useSettingsStore = create<SettingsState>((set) => ({
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

  resetToDefaults: () => {
    const settings = { ...DEFAULTS };
    saveSettings(settings);
    set({ settings });
  },
}));
