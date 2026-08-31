import { setNativeTheme } from "../tauri-commands";
import type { ResolvedTheme } from "./theme-types";

const THEME_STORAGE_KEY = "deathpush:theme";

type ApplyThemeOptions = {
  transient?: boolean;
};

export const applyTheme = (theme: ResolvedTheme, options: ApplyThemeOptions = {}): void => {
  const root = document.documentElement;

  const staleVars: string[] = [];
  for (let i = 0; i < root.style.length; i++) {
    const prop = root.style[i];
    if (prop.startsWith("--vscode-")) staleVars.push(prop);
  }
  for (const prop of staleVars) root.style.removeProperty(prop);

  for (const [key, value] of Object.entries(theme.colors)) {
    const cssVar = `--vscode-${key.split(".").join("-")}`;
    root.style.setProperty(cssVar, value);
  }

  const scheme = theme.kind;
  root.style.setProperty("color-scheme", scheme);
  root.dataset.colorScheme = scheme;

  if (!options.transient) {
    const isDark = scheme === "dark";
    setNativeTheme(isDark).catch(() => {});
  }

  window.dispatchEvent(new CustomEvent("deathpush:theme-applied", { detail: { colors: theme.colors } }));

  if (!options.transient) {
    localStorage.setItem(THEME_STORAGE_KEY, theme.id);
  }
};

export type TerminalTheme = {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
};

const expandHexColor = (value: string): string => {
  const short = /^#([0-9a-fA-F]{3})$/.exec(value);
  if (short) {
    const [r, g, b] = short[1];
    return `#${r}${r}${g}${g}${b}${b}`;
  }
  const withAlpha = /^#([0-9a-fA-F]{8})$/.exec(value);
  if (withAlpha) return `#${withAlpha[1].slice(0, 6)}`;
  return value;
};

export const getTerminalTheme = (colors: Record<string, string>): TerminalTheme => ({
  background: expandHexColor(colors["terminal.background"] ?? colors["editor.background"] ?? "#1E1E1E"),
  foreground: expandHexColor(colors["terminal.foreground"] ?? colors["editor.foreground"] ?? "#CCCCCC"),
  cursor: expandHexColor(colors["terminalCursor.foreground"] ?? "#AEAFAD"),
  cursorAccent: expandHexColor(colors["terminalCursor.background"] ?? "#000000"),
  selectionBackground: colors["terminal.selectionBackground"] ?? "rgba(255, 255, 255, 0.3)",
  black: expandHexColor(colors["terminal.ansiBlack"] ?? "#000000"),
  red: expandHexColor(colors["terminal.ansiRed"] ?? "#CD3131"),
  green: expandHexColor(colors["terminal.ansiGreen"] ?? "#0DBC79"),
  yellow: expandHexColor(colors["terminal.ansiYellow"] ?? "#E5E510"),
  blue: expandHexColor(colors["terminal.ansiBlue"] ?? "#2472C8"),
  magenta: expandHexColor(colors["terminal.ansiMagenta"] ?? "#BC3FBC"),
  cyan: expandHexColor(colors["terminal.ansiCyan"] ?? "#11A8CD"),
  white: expandHexColor(colors["terminal.ansiWhite"] ?? "#E5E5E5"),
  brightBlack: expandHexColor(colors["terminal.ansiBrightBlack"] ?? "#666666"),
  brightRed: expandHexColor(colors["terminal.ansiBrightRed"] ?? "#F14C4C"),
  brightGreen: expandHexColor(colors["terminal.ansiBrightGreen"] ?? "#23D18B"),
  brightYellow: expandHexColor(colors["terminal.ansiBrightYellow"] ?? "#F5F543"),
  brightBlue: expandHexColor(colors["terminal.ansiBrightBlue"] ?? "#3B8EEA"),
  brightMagenta: expandHexColor(colors["terminal.ansiBrightMagenta"] ?? "#D670D6"),
  brightCyan: expandHexColor(colors["terminal.ansiBrightCyan"] ?? "#29B8DB"),
  brightWhite: expandHexColor(colors["terminal.ansiBrightWhite"] ?? "#E5E5E5"),
});
