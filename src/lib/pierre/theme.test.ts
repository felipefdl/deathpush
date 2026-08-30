import { describe, it, expect, vi } from "vite-plus/test";
import type { ResolvedTheme, ThemeKind } from "../themes/theme-types";

const { registerCustomThemeMock } = vi.hoisted(() => ({
  registerCustomThemeMock: vi.fn(),
}));

vi.mock("@pierre/diffs", () => ({
  registerCustomTheme: registerCustomThemeMock,
}));

import { registerDeathPushPierreTheme } from "./theme";

const uiThemeByKind = {
  dark: "vs-dark",
  light: "vs",
  "hc-dark": "hc-black",
  "hc-light": "hc-light",
} as const;

const theme = (kind: ThemeKind, id = "deathpush-theme"): ResolvedTheme => ({
  id,
  label: "DeathPush Display Label",
  uiTheme: uiThemeByKind[kind],
  kind,
  colors: { "editor.background": "#010203", "editor.foreground": "#CCCCCC" },
  tokenColors: [{ scope: "comment", settings: { foreground: "#6A9955" } }],
});

const loadRegisteredTheme = async (kind: ThemeKind, id?: string) => {
  registerCustomThemeMock.mockClear();
  const resolved = theme(kind, id);
  await registerDeathPushPierreTheme(resolved);
  expect(registerCustomThemeMock).toHaveBeenCalledTimes(1);
  expect(registerCustomThemeMock).toHaveBeenCalledWith(resolved.id, expect.any(Function));
  const loader = registerCustomThemeMock.mock.calls[0]?.[1] as () => Promise<{
    name: string;
    type: "dark" | "light";
    bg: string;
    fg: string;
    colors: ResolvedTheme["colors"];
    tokenColors: ResolvedTheme["tokenColors"];
  }>;
  return { resolved, loaded: await loader() };
};

describe("registerDeathPushPierreTheme", () => {
  it("registers theme.id and never the display label", async () => {
    const { resolved, loaded } = await loadRegisteredTheme("dark", "preview-theme");
    expect(loaded.name).toBe("preview-theme");
    expect(loaded.name).not.toBe(resolved.label);
    expect(loaded.colors).toEqual(resolved.colors);
    expect(loaded.tokenColors.slice(0, resolved.tokenColors.length)).toEqual(resolved.tokenColors);
  });

  it.each([
    ["dark", ["#9CDCFE", "#CE9178", "#B5CEA8", "#CE9178"]],
    ["light", ["#A31515", "#0451A5", "#098658", "#0451A5"]],
    ["hc-dark", ["#9CDCFE", "#CE9178", "#FFFFFF", "#569CD6"]],
    ["hc-light", ["#A31515", "#0451A5", "#098658", "#0000FF"]],
  ] as const)("preserves Monaco JSON colors for %s themes", async (kind, foregrounds) => {
    const { loaded } = await loadRegisteredTheme(kind);
    expect(loaded.tokenColors.slice(-4)).toEqual([
      { scope: "support.type.property-name.json", settings: { foreground: foregrounds[0] } },
      { scope: "string.quoted.double.json", settings: { foreground: foregrounds[1] } },
      { scope: "constant.numeric.json", settings: { foreground: foregrounds[2] } },
      { scope: "constant.language.json", settings: { foreground: foregrounds[3] } },
    ]);
  });

  it("uses the editor surface as Pierre's base colors", async () => {
    const { loaded } = await loadRegisteredTheme("dark");
    expect(loaded.bg).toBe("#010203");
    expect(loaded.fg).toBe("#CCCCCC");
  });

  it("maps light and hc-light kinds to type light", async () => {
    expect((await loadRegisteredTheme("light")).loaded.type).toBe("light");
    expect((await loadRegisteredTheme("hc-light")).loaded.type).toBe("light");
  });

  it("maps dark and hc-dark kinds to type dark", async () => {
    expect((await loadRegisteredTheme("dark")).loaded.type).toBe("dark");
    expect((await loadRegisteredTheme("hc-dark")).loaded.type).toBe("dark");
  });
});
