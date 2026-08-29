import { describe, it, expect, vi } from "vite-plus/test";
import type { ResolvedTheme, ThemeKind } from "../themes/theme-types";

const { registerCustomThemeMock } = vi.hoisted(() => ({
  registerCustomThemeMock: vi.fn(),
}));

vi.mock("@pierre/diffs", () => ({
  registerCustomTheme: registerCustomThemeMock,
}));

import { registerDeathPushPierreTheme } from "./theme";

const theme = (kind: ThemeKind, id = "deathpush-theme"): ResolvedTheme => ({
  id,
  label: "DeathPush Display Label",
  uiTheme: kind === "light" || kind === "hc-light" ? "vs" : "vs-dark",
  kind,
  colors: { "editor.background": "#010203" },
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
    expect(loaded.tokenColors).toEqual(resolved.tokenColors);
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
