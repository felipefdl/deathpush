import { describe, expect, it } from "vite-plus/test";
import {
  DEFAULT_DARK_THEME_ID,
  DEFAULT_LIGHT_THEME_ID,
  THEME_ENTRIES,
  getDefaultResolvedTheme,
  getResolvedTheme,
} from "./theme-registry";

describe("Shiki theme registry", () => {
  it("uses Vesper for dark mode and Ayu for light mode by default", () => {
    expect(DEFAULT_DARK_THEME_ID).toBe("vesper");
    expect(DEFAULT_LIGHT_THEME_ID).toBe("ayu-light");
    expect(getDefaultResolvedTheme("dark").id).toBe("vesper");
    expect(getDefaultResolvedTheme("light").id).toBe("ayu-light");
  });

  it("exposes the complete Shiki catalog without legacy themes", () => {
    expect(THEME_ENTRIES.length).toBeGreaterThan(50);
    expect(THEME_ENTRIES).toContainEqual(expect.objectContaining({ id: "vesper", kind: "dark" }));
    expect(THEME_ENTRIES).toContainEqual(expect.objectContaining({ id: "ayu-light", kind: "light" }));
    expect(THEME_ENTRIES.some((entry) => entry.id.startsWith("deathayu"))).toBe(false);
    expect(new Set(THEME_ENTRIES.map((entry) => entry.kind))).toEqual(new Set(["dark", "light"]));
  });

  it("resolves and normalizes Shiki workbench colors", async () => {
    const theme = await getResolvedTheme("ayu-dark");
    expect(theme?.type).toBe("dark");
    expect(theme?.colors["editor.background"]).toBeTruthy();
    expect(theme?.colors["sideBar.background"]).toBeTruthy();
    expect(theme?.colors["list.hoverBackground"]).toBeTruthy();
  });

  it("returns undefined for an unknown theme", async () => {
    await expect(getResolvedTheme("not-a-theme")).resolves.toBeUndefined();
  });

  it("caches resolved themes", async () => {
    const first = await getResolvedTheme("ayu-light");
    const second = await getResolvedTheme("ayu-light");
    expect(first).toBe(second);
  });
});
