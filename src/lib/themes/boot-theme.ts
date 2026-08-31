import ayuLight from "@shikijs/themes/ayu-light";
import vesper from "@shikijs/themes/vesper";
import { normalizeThemeColors } from "@pierre/theming/color";
import type { ResolvedTheme, ThemeEntry, ThemeKind, TokenColor } from "./theme-types";

export const DEFAULT_DARK_THEME_ID = "vesper";
export const DEFAULT_LIGHT_THEME_ID = "ayu-light";

export type ShikiTheme = {
  name?: string;
  type?: ThemeKind;
  bg?: string;
  fg?: string;
  colors?: Record<string, string>;
  tokenColors?: TokenColor[];
};

export const resolveTheme = (entry: ThemeEntry, rawTheme: ShikiTheme): ResolvedTheme => {
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

const darkTheme = resolveTheme({ id: DEFAULT_DARK_THEME_ID, label: "Vesper", kind: "dark" }, vesper);
const lightTheme = resolveTheme({ id: DEFAULT_LIGHT_THEME_ID, label: "Ayu Light", kind: "light" }, ayuLight);

export const getBootTheme = (
  kind: ThemeKind = window.matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light"
): ResolvedTheme => (kind === "dark" ? darkTheme : lightTheme);
