import { createThemeCatalog } from "@pierre/theming";
import { shikiThemes } from "@pierre/theming/themes";
import type { ShikiTheme } from "./boot-theme";
import { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID, getBootTheme, resolveTheme } from "./boot-theme";
import type { ResolvedTheme, ThemeEntry, ThemeKind } from "./theme-types";
export { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID };

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

const resolvedCache = new Map<string, ResolvedTheme>();

const defaultDarkTheme = getBootTheme("dark");
const defaultLightTheme = getBootTheme("light");
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
