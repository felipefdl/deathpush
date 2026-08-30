import { createThemeCatalog } from "@pierre/theming";
import { normalizeThemeColors } from "@pierre/theming/color";
import { shikiThemes } from "@pierre/theming/themes";
import vesper from "@shikijs/themes/vesper";
import ayuLight from "@shikijs/themes/ayu-light";
import type { ResolvedTheme, ThemeEntry, ThemeKind, TokenColor } from "./theme-types";

export const DEFAULT_DARK_THEME_ID = "vesper";
export const DEFAULT_LIGHT_THEME_ID = "ayu-light";

const catalog = createThemeCatalog({
  themes: shikiThemes,
  defaultDarkThemeName: DEFAULT_DARK_THEME_ID,
  defaultLightThemeName: DEFAULT_LIGHT_THEME_ID,
});

const labelForTheme = (name: string, displayName?: string): string =>
  displayName ??
  name
    .split("-")
    .map((part) => `${part.charAt(0).toUpperCase()}${part.slice(1)}`)
    .join(" ");

export const THEME_ENTRIES: ThemeEntry[] = catalog.getThemes().map((descriptor) => ({
  id: descriptor.name,
  label: labelForTheme(descriptor.name, descriptor.displayName),
  kind: descriptor.colorScheme ?? "dark",
}));

const entriesById = new Map(THEME_ENTRIES.map((entry) => [entry.id, entry]));

type ShikiTheme = {
  name?: string;
  type?: ThemeKind;
  bg?: string;
  fg?: string;
  colors?: Record<string, string>;
  tokenColors?: TokenColor[];
};

const resolveTheme = (entry: ThemeEntry, rawTheme: ShikiTheme): ResolvedTheme => {
  const normalized = normalizeThemeColors(rawTheme);
  const colors = normalized.colors ?? rawTheme.colors ?? {};
  const bg =
    normalized.bg ?? rawTheme.bg ?? colors["editor.background"] ?? (entry.kind === "dark" ? "#000000" : "#ffffff");
  const fg =
    normalized.fg ?? rawTheme.fg ?? colors["editor.foreground"] ?? (entry.kind === "dark" ? "#ffffff" : "#000000");
  return {
    id: entry.id,
    label: entry.label,
    kind: entry.kind,
    name: rawTheme.name ?? entry.id,
    type: rawTheme.type ?? entry.kind,
    bg,
    fg,
    colors: {
      ...colors,
      "editor.background": colors["editor.background"] ?? bg,
      "editor.foreground": colors["editor.foreground"] ?? fg,
    },
    tokenColors: rawTheme.tokenColors ?? [],
  };
};

const resolvedCache = new Map<string, ResolvedTheme>();

const defaultDarkTheme = resolveTheme(entriesById.get(DEFAULT_DARK_THEME_ID)!, vesper);
const defaultLightTheme = resolveTheme(entriesById.get(DEFAULT_LIGHT_THEME_ID)!, ayuLight);
resolvedCache.set(DEFAULT_DARK_THEME_ID, defaultDarkTheme);
resolvedCache.set(DEFAULT_LIGHT_THEME_ID, defaultLightTheme);

export const getDefaultResolvedTheme = (kind: ThemeKind): ResolvedTheme =>
  kind === "dark" ? defaultDarkTheme : defaultLightTheme;

export const getResolvedTheme = async (id: string): Promise<ResolvedTheme | undefined> => {
  const cached = resolvedCache.get(id);
  if (cached) return cached;
  const descriptor = catalog.getTheme(id);
  const entry = entriesById.get(id);
  if (!descriptor || !entry) return undefined;
  const theme = resolveTheme(entry, (await descriptor.load()) as ShikiTheme);
  resolvedCache.set(id, theme);
  return theme;
};
