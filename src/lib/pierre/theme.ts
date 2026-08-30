import { registerCustomTheme } from "@pierre/diffs";
import type { ResolvedTheme, ThemeKind, TokenColor, UiTheme } from "../themes/theme-types";

export const pierreThemeType = (kind: ThemeKind): "light" | "dark" =>
  kind === "light" || kind === "hc-light" ? "light" : "dark";

const MONACO_JSON_SCOPES = [
  "support.type.property-name.json",
  "string.quoted.double.json",
  "constant.numeric.json",
  "constant.language.json",
] as const;

const MONACO_JSON_FOREGROUNDS: Record<UiTheme, readonly [string, string, string, string]> = {
  "vs-dark": ["#9CDCFE", "#CE9178", "#B5CEA8", "#CE9178"],
  vs: ["#A31515", "#0451A5", "#098658", "#0451A5"],
  "hc-black": ["#9CDCFE", "#CE9178", "#FFFFFF", "#569CD6"],
  "hc-light": ["#A31515", "#0451A5", "#098658", "#0000FF"],
};

export const pierreTokenColors = (theme: ResolvedTheme): TokenColor[] => [
  ...theme.tokenColors,
  ...MONACO_JSON_SCOPES.map((scope, index) => ({
    scope,
    settings: { foreground: MONACO_JSON_FOREGROUNDS[theme.uiTheme][index] },
  })),
];

export const registerDeathPushPierreTheme = async (theme: ResolvedTheme): Promise<void> => {
  registerCustomTheme(theme.id, async () => ({
    name: theme.id,
    type: pierreThemeType(theme.kind),
    bg: theme.colors["editor.background"],
    fg: theme.colors["editor.foreground"],
    colors: theme.colors,
    tokenColors: pierreTokenColors(theme),
  }));
};
