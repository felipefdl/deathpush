import { registerCustomTheme } from "@pierre/diffs";
import type { ResolvedTheme, ThemeKind } from "../themes/theme-types";

const pierreThemeType = (kind: ThemeKind): "light" | "dark" =>
  kind === "light" || kind === "hc-light" ? "light" : "dark";

export const registerDeathPushPierreTheme = async (theme: ResolvedTheme): Promise<void> => {
  registerCustomTheme(theme.id, async () => ({
    name: theme.id,
    type: pierreThemeType(theme.kind),
    colors: theme.colors,
    tokenColors: theme.tokenColors,
  }));
};
