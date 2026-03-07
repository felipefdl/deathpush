import type { ThemeEntry, VscodeThemeJson, ResolvedTheme, TokenColor } from "./theme-types";
import { DEFAULT_DARK_COLORS, DEFAULT_LIGHT_COLORS } from "./defaults";

import darkVsJson from "./json/dark_vs.json";
import darkPlusJson from "./json/dark_plus.json";
import darkModernJson from "./json/dark_modern.json";
import lightVsJson from "./json/light_vs.json";
import lightPlusJson from "./json/light_plus.json";
import lightModernJson from "./json/light_modern.json";
import hcBlackJson from "./json/hc_black.json";
import hcLightJson from "./json/hc_light.json";
import monokaiJson from "./json/monokai-color-theme.json";
import dimmedMonokaiJson from "./json/dimmed-monokai-color-theme.json";
import solarizedDarkJson from "./json/solarized-dark-color-theme.json";
import solarizedLightJson from "./json/solarized-light-color-theme.json";
import abyssJson from "./json/abyss-color-theme.json";
import kimbieDarkJson from "./json/kimbie-dark-color-theme.json";
import quietLightJson from "./json/quietlight-color-theme.json";
import redJson from "./json/Red-color-theme.json";
import tomorrowNightBlueJson from "./json/tomorrow-night-blue-color-theme.json";
import nordJson from "./json/nord.json";
import oneDarkProJson from "./json/one-dark-pro.json";
import draculaJson from "./json/dracula.json";
import draculaSoftJson from "./json/dracula-soft.json";
import deathayuDarkJson from "./json/deathayu-dark.json";
import deathayuLightJson from "./json/deathayu-light.json";
import ayuDarkJson from "./json/ayu-dark.json";
import ayuLightJson from "./json/ayu-light.json";
import ayuMirageJson from "./json/ayu-mirage.json";
import catppuccinMochaJson from "./json/catppuccin-mocha.json";
import catppuccinLatteJson from "./json/catppuccin-latte.json";
import catppuccinFrappeJson from "./json/catppuccin-frappe.json";
import catppuccinMacchiatoJson from "./json/catppuccin-macchiato.json";

const INCLUDE_MAP: Record<string, VscodeThemeJson> = {
  "./dark_vs.json": darkVsJson as VscodeThemeJson,
  "./dark_plus.json": darkPlusJson as VscodeThemeJson,
  "./light_vs.json": lightVsJson as VscodeThemeJson,
  "./light_plus.json": lightPlusJson as VscodeThemeJson,
};

interface ThemeRegistration {
  entry: ThemeEntry;
  json: VscodeThemeJson;
}

const THEMES: ThemeRegistration[] = [
  { entry: { id: "dark-modern", label: "Dark Modern", uiTheme: "vs-dark", kind: "dark" }, json: darkModernJson as VscodeThemeJson },
  { entry: { id: "dark-plus", label: "Dark+", uiTheme: "vs-dark", kind: "dark" }, json: darkPlusJson as VscodeThemeJson },
  { entry: { id: "dark-vs", label: "Dark (Visual Studio)", uiTheme: "vs-dark", kind: "dark" }, json: darkVsJson as VscodeThemeJson },
  { entry: { id: "abyss", label: "Abyss", uiTheme: "vs-dark", kind: "dark" }, json: abyssJson as VscodeThemeJson },
  { entry: { id: "kimbie-dark", label: "Kimbie Dark", uiTheme: "vs-dark", kind: "dark" }, json: kimbieDarkJson as VscodeThemeJson },
  { entry: { id: "monokai", label: "Monokai", uiTheme: "vs-dark", kind: "dark" }, json: monokaiJson as VscodeThemeJson },
  { entry: { id: "monokai-dimmed", label: "Monokai Dimmed", uiTheme: "vs-dark", kind: "dark" }, json: dimmedMonokaiJson as VscodeThemeJson },
  { entry: { id: "nord", label: "Nord", uiTheme: "vs-dark", kind: "dark" }, json: nordJson as VscodeThemeJson },
  { entry: { id: "one-dark-pro", label: "One Dark Pro", uiTheme: "vs-dark", kind: "dark" }, json: oneDarkProJson as VscodeThemeJson },
  { entry: { id: "red", label: "Red", uiTheme: "vs-dark", kind: "dark" }, json: redJson as VscodeThemeJson },
  { entry: { id: "solarized-dark", label: "Solarized Dark", uiTheme: "vs-dark", kind: "dark" }, json: solarizedDarkJson as VscodeThemeJson },
  { entry: { id: "tomorrow-night-blue", label: "Tomorrow Night Blue", uiTheme: "vs-dark", kind: "dark" }, json: tomorrowNightBlueJson as VscodeThemeJson },
  { entry: { id: "dracula", label: "Dracula", uiTheme: "vs-dark", kind: "dark" }, json: draculaJson as VscodeThemeJson },
  { entry: { id: "dracula-soft", label: "Dracula Soft", uiTheme: "vs-dark", kind: "dark" }, json: draculaSoftJson as VscodeThemeJson },
  { entry: { id: "deathayu-dark", label: "Death Ayu Dark", description: "Default Dark Death", uiTheme: "vs-dark", kind: "dark" }, json: deathayuDarkJson as VscodeThemeJson },
  { entry: { id: "ayu-dark", label: "Ayu Dark", uiTheme: "vs-dark", kind: "dark" }, json: ayuDarkJson as VscodeThemeJson },
  { entry: { id: "ayu-mirage", label: "Ayu Mirage", uiTheme: "vs-dark", kind: "dark" }, json: ayuMirageJson as VscodeThemeJson },
  { entry: { id: "catppuccin-mocha", label: "Catppuccin Mocha", uiTheme: "vs-dark", kind: "dark" }, json: catppuccinMochaJson as VscodeThemeJson },
  { entry: { id: "catppuccin-frappe", label: "Catppuccin Frappe", uiTheme: "vs-dark", kind: "dark" }, json: catppuccinFrappeJson as VscodeThemeJson },
  { entry: { id: "catppuccin-macchiato", label: "Catppuccin Macchiato", uiTheme: "vs-dark", kind: "dark" }, json: catppuccinMacchiatoJson as VscodeThemeJson },
  { entry: { id: "light-modern", label: "Light Modern", uiTheme: "vs", kind: "light" }, json: lightModernJson as VscodeThemeJson },
  { entry: { id: "light-plus", label: "Light+", uiTheme: "vs", kind: "light" }, json: lightPlusJson as VscodeThemeJson },
  { entry: { id: "light-vs", label: "Light (Visual Studio)", uiTheme: "vs", kind: "light" }, json: lightVsJson as VscodeThemeJson },
  { entry: { id: "quiet-light", label: "Quiet Light", uiTheme: "vs", kind: "light" }, json: quietLightJson as VscodeThemeJson },
  { entry: { id: "solarized-light", label: "Solarized Light", uiTheme: "vs", kind: "light" }, json: solarizedLightJson as VscodeThemeJson },
  { entry: { id: "deathayu-light", label: "Death Ayu Light", description: "Default Light Death", uiTheme: "vs", kind: "light" }, json: deathayuLightJson as VscodeThemeJson },
  { entry: { id: "ayu-light", label: "Ayu Light", uiTheme: "vs", kind: "light" }, json: ayuLightJson as VscodeThemeJson },
  { entry: { id: "catppuccin-latte", label: "Catppuccin Latte", uiTheme: "vs", kind: "light" }, json: catppuccinLatteJson as VscodeThemeJson },
  { entry: { id: "hc-black", label: "High Contrast Dark", uiTheme: "hc-black", kind: "hc-dark" }, json: hcBlackJson as VscodeThemeJson },
  { entry: { id: "hc-light", label: "High Contrast Light", uiTheme: "hc-light", kind: "hc-light" }, json: hcLightJson as VscodeThemeJson },
];

const resolveIncludeChain = (json: VscodeThemeJson): { colors: Record<string, string>; tokenColors: TokenColor[] } => {
  let baseColors: Record<string, string> = {};
  let baseTokenColors: TokenColor[] = [];

  if (json.include) {
    const baseJson = INCLUDE_MAP[json.include];
    if (baseJson) {
      const resolved = resolveIncludeChain(baseJson);
      baseColors = resolved.colors;
      baseTokenColors = resolved.tokenColors;
    }
  }

  const colors = { ...baseColors, ...json.colors };
  const tokenColors = mergeTokenColors(baseTokenColors, json.tokenColors ?? []);

  return { colors, tokenColors };
};

const mergeTokenColors = (base: TokenColor[], overlay: TokenColor[]): TokenColor[] => {
  const result = [...base];
  for (const tc of overlay) {
    const scopes = normalizeScope(tc.scope);
    const existingIdx = result.findIndex((r) => {
      const rScopes = normalizeScope(r.scope);
      return rScopes.length === scopes.length && rScopes.every((s, i) => s === scopes[i]);
    });
    if (existingIdx >= 0) {
      result[existingIdx] = { ...result[existingIdx], settings: { ...result[existingIdx].settings, ...tc.settings } };
    } else {
      result.push(tc);
    }
  }
  return result;
};

const normalizeScope = (scope: string | string[] | undefined): string[] => {
  if (!scope) return [];
  return Array.isArray(scope) ? scope : [scope];
};

const getDefaults = (uiTheme: string): Record<string, string> => {
  if (uiTheme === "vs") return DEFAULT_LIGHT_COLORS;
  if (uiTheme === "hc-light") return DEFAULT_LIGHT_COLORS;
  return DEFAULT_DARK_COLORS;
};

export const resolveTheme = (entry: ThemeEntry, json: VscodeThemeJson): ResolvedTheme => {
  const { colors, tokenColors } = resolveIncludeChain(json);
  const defaults = getDefaults(entry.uiTheme);
  const mergedColors = { ...defaults, ...colors };

  // Force terminal background to match editor background
  delete mergedColors["terminal.background"];

  return {
    ...entry,
    colors: mergedColors,
    tokenColors,
  };
};

export const THEME_ENTRIES: ThemeEntry[] = THEMES.map((t) => t.entry);

const resolvedCache = new Map<string, ResolvedTheme>();

export const getResolvedTheme = (id: string): ResolvedTheme | undefined => {
  const cached = resolvedCache.get(id);
  if (cached) return cached;

  const registration = THEMES.find((t) => t.entry.id === id);
  if (!registration) return undefined;

  const resolved = resolveTheme(registration.entry, registration.json);
  resolvedCache.set(id, resolved);
  return resolved;
};

export const DEFAULT_DARK_THEME_ID = "deathayu-dark";
export const DEFAULT_LIGHT_THEME_ID = "deathayu-light";
