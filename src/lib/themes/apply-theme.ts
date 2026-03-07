import { loader } from "@monaco-editor/react";
import type { ResolvedTheme, UiTheme } from "./theme-types";

const THEME_STORAGE_KEY = "deathpush:theme";

const uiThemeToMonacoBase = (uiTheme: UiTheme): "vs" | "vs-dark" | "hc-black" | "hc-light" => {
  switch (uiTheme) {
    case "vs": return "vs";
    case "vs-dark": return "vs-dark";
    case "hc-black": return "hc-black";
    case "hc-light": return "hc-light";
  }
};

export const applyTheme = (theme: ResolvedTheme): void => {
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

  const scheme = theme.kind === "dark" || theme.kind === "hc-dark" ? "dark" : "light";
  root.style.setProperty("color-scheme", scheme);

  document.body.classList.remove("vs", "hc-black", "hc-light");
  if (theme.uiTheme === "vs") document.body.classList.add("vs");
  if (theme.uiTheme === "hc-black") document.body.classList.add("hc-black");
  if (theme.uiTheme === "hc-light") document.body.classList.add("hc-light");

  applyMonacoTheme(theme);

  window.dispatchEvent(new CustomEvent("deathpush:theme-applied", { detail: { colors: theme.colors } }));

  localStorage.setItem(THEME_STORAGE_KEY, theme.id);
};

const applyMonacoTheme = (theme: ResolvedTheme): void => {
  loader.init().then((monaco) => {
    const base = uiThemeToMonacoBase(theme.uiTheme);

    const rules = theme.tokenColors.flatMap((tc) => {
      const scopes = Array.isArray(tc.scope) ? tc.scope : tc.scope ? [tc.scope] : [];
      return scopes.map((scope) => ({
        token: scope,
        foreground: tc.settings.foreground?.replace("#", ""),
        background: tc.settings.background?.replace("#", ""),
        fontStyle: tc.settings.fontStyle,
      }));
    });

    const colors: Record<string, string> = {};
    for (const [key, value] of Object.entries(theme.colors)) {
      colors[key] = value;
    }

    monaco.editor.defineTheme(theme.id, { base, inherit: true, rules, colors });
    monaco.editor.setTheme(theme.id);
  });
};

export const getTerminalTheme = (colors: Record<string, string>) => ({
  background: colors["terminal.background"] ?? colors["editor.background"] ?? "#1E1E1E",
  foreground: colors["terminal.foreground"] ?? colors["editor.foreground"] ?? "#CCCCCC",
  cursor: colors["terminalCursor.foreground"] ?? "#AEAFAD",
  cursorAccent: colors["terminalCursor.background"] ?? "#000000",
  selectionBackground: colors["terminal.selectionBackground"] ?? "rgba(255, 255, 255, 0.3)",
  black: colors["terminal.ansiBlack"] ?? "#000000",
  red: colors["terminal.ansiRed"] ?? "#CD3131",
  green: colors["terminal.ansiGreen"] ?? "#0DBC79",
  yellow: colors["terminal.ansiYellow"] ?? "#E5E510",
  blue: colors["terminal.ansiBlue"] ?? "#2472C8",
  magenta: colors["terminal.ansiMagenta"] ?? "#BC3FBC",
  cyan: colors["terminal.ansiCyan"] ?? "#11A8CD",
  white: colors["terminal.ansiWhite"] ?? "#E5E5E5",
  brightBlack: colors["terminal.ansiBrightBlack"] ?? "#666666",
  brightRed: colors["terminal.ansiBrightRed"] ?? "#F14C4C",
  brightGreen: colors["terminal.ansiBrightGreen"] ?? "#23D18B",
  brightYellow: colors["terminal.ansiBrightYellow"] ?? "#F5F543",
  brightBlue: colors["terminal.ansiBrightBlue"] ?? "#3B8EEA",
  brightMagenta: colors["terminal.ansiBrightMagenta"] ?? "#D670D6",
  brightCyan: colors["terminal.ansiBrightCyan"] ?? "#29B8DB",
  brightWhite: colors["terminal.ansiBrightWhite"] ?? "#E5E5E5",
});
