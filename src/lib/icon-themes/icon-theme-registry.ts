import type { IconThemeEntry, IconThemeJson, ResolvedIconTheme } from "./icon-theme-types";
import { generateIconThemeCss } from "./generate-icon-css";

import setiJson from "./json/vs-seti-icon-theme.json";
import materialJson from "./json/material-icon-theme.json";

interface IconThemeRegistration {
  entry: IconThemeEntry;
  json: IconThemeJson | null;
  assetBasePath: string;
}

const ICON_THEMES: IconThemeRegistration[] = [
  { entry: { id: "none", label: "None" }, json: null, assetBasePath: "" },
  { entry: { id: "material", label: "Material Icon Theme" }, json: materialJson as unknown as IconThemeJson, assetBasePath: "/icon-themes/material" },
  { entry: { id: "seti", label: "Seti (Visual Studio Code)" }, json: setiJson as unknown as IconThemeJson, assetBasePath: "/icon-themes/seti" },
];

export const DEFAULT_ICON_THEME_ID = "material";

export const ICON_THEME_ENTRIES: IconThemeEntry[] = ICON_THEMES.map((t) => t.entry);

const NONE_THEME: ResolvedIconTheme = {
  id: "none",
  label: "None",
  cssContent: "",
  hasFileIcons: false,
  hasFolderIcons: false,
  hidesExplorerArrows: false,
};

const resolvedCache = new Map<string, ResolvedIconTheme>();

export const getResolvedIconTheme = (id: string): ResolvedIconTheme | undefined => {
  if (id === "none") return NONE_THEME;

  const cached = resolvedCache.get(id);
  if (cached) return cached;

  const registration = ICON_THEMES.find((t) => t.entry.id === id);
  if (!registration || !registration.json) return undefined;

  const result = generateIconThemeCss(registration.json, registration.assetBasePath);

  const resolved: ResolvedIconTheme = {
    ...registration.entry,
    cssContent: result.cssContent,
    hasFileIcons: result.hasFileIcons,
    hasFolderIcons: result.hasFolderIcons,
    hidesExplorerArrows: result.hidesExplorerArrows,
  };

  resolvedCache.set(id, resolved);
  return resolved;
};
