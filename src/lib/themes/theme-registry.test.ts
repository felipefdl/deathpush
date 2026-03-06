import { describe, it, expect } from "vitest";
import {
  resolveTheme,
  THEME_ENTRIES,
  getResolvedTheme,
  DEFAULT_DARK_THEME_ID,
  DEFAULT_LIGHT_THEME_ID,
} from "./theme-registry";
import { DEFAULT_DARK_COLORS, DEFAULT_LIGHT_COLORS } from "./defaults";
import type { VscodeThemeJson, ThemeEntry } from "./theme-types";

const darkEntry: ThemeEntry = { id: "test-dark", label: "Test Dark", uiTheme: "vs-dark", kind: "dark" };
const lightEntry: ThemeEntry = { id: "test-light", label: "Test Light", uiTheme: "vs", kind: "light" };
const hcLightEntry: ThemeEntry = { id: "test-hc-light", label: "Test HC Light", uiTheme: "hc-light", kind: "hc-light" };

describe("resolveTheme", () => {
  it("returns colors and tokenColors from a simple theme", () => {
    const json: VscodeThemeJson = {
      colors: { "editor.background": "#112233" },
      tokenColors: [{ scope: "comment", settings: { foreground: "#666" } }],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.colors["editor.background"]).toBe("#112233");
    expect(result.tokenColors).toHaveLength(1);
    expect(result.tokenColors[0].settings.foreground).toBe("#666");
  });

  it("inherits DEFAULT_DARK_COLORS for vs-dark theme", () => {
    const json: VscodeThemeJson = { colors: {}, tokenColors: [] };
    const result = resolveTheme(darkEntry, json);
    expect(result.colors["editor.background"]).toBe(DEFAULT_DARK_COLORS["editor.background"]);
  });

  it("inherits DEFAULT_LIGHT_COLORS for vs theme", () => {
    const json: VscodeThemeJson = { colors: {}, tokenColors: [] };
    const result = resolveTheme(lightEntry, json);
    expect(result.colors["editor.background"]).toBe(DEFAULT_LIGHT_COLORS["editor.background"]);
  });

  it("inherits DEFAULT_LIGHT_COLORS for hc-light theme", () => {
    const json: VscodeThemeJson = { colors: {}, tokenColors: [] };
    const result = resolveTheme(hcLightEntry, json);
    expect(result.colors["editor.background"]).toBe(DEFAULT_LIGHT_COLORS["editor.background"]);
  });

  it("overrides default colors with provided colors", () => {
    const json: VscodeThemeJson = { colors: { "editor.background": "#CUSTOM" }, tokenColors: [] };
    const result = resolveTheme(darkEntry, json);
    expect(result.colors["editor.background"]).toBe("#CUSTOM");
  });

  it("includes entry fields in the result", () => {
    const json: VscodeThemeJson = { colors: {}, tokenColors: [] };
    const result = resolveTheme(darkEntry, json);
    expect(result.id).toBe("test-dark");
    expect(result.label).toBe("Test Dark");
    expect(result.uiTheme).toBe("vs-dark");
    expect(result.kind).toBe("dark");
  });
});

describe("getResolvedTheme", () => {
  it("returns a resolved theme for a valid id", () => {
    const result = getResolvedTheme(DEFAULT_DARK_THEME_ID);
    expect(result).toBeDefined();
    expect(result!.id).toBe(DEFAULT_DARK_THEME_ID);
    expect(result!.colors).toBeDefined();
    expect(result!.tokenColors).toBeDefined();
  });

  it("returns undefined for an unknown id", () => {
    const result = getResolvedTheme("nonexistent-theme-id");
    expect(result).toBeUndefined();
  });

  it("returns the same cached reference on repeated calls", () => {
    const first = getResolvedTheme(DEFAULT_LIGHT_THEME_ID);
    const second = getResolvedTheme(DEFAULT_LIGHT_THEME_ID);
    expect(first).toBe(second);
  });
});

describe("THEME_ENTRIES", () => {
  it("has the correct number of themes", () => {
    expect(THEME_ENTRIES).toHaveLength(30);
  });

  it("every entry has required fields", () => {
    for (const entry of THEME_ENTRIES) {
      expect(entry.id).toBeTruthy();
      expect(entry.label).toBeTruthy();
      expect(entry.uiTheme).toBeTruthy();
      expect(entry.kind).toBeTruthy();
    }
  });
});

describe("normalizeScope (tested through resolveTheme)", () => {
  it("handles tokenColor with no scope", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [{ settings: { foreground: "#FFF" } }],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.tokenColors).toHaveLength(1);
    expect(result.tokenColors[0].scope).toBeUndefined();
  });

  it("handles tokenColor with string scope", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [{ scope: "comment", settings: { foreground: "#888" } }],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.tokenColors[0].scope).toBe("comment");
  });

  it("handles tokenColor with array scope", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [{ scope: ["comment", "string"], settings: { foreground: "#999" } }],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.tokenColors[0].scope).toEqual(["comment", "string"]);
  });
});

describe("mergeTokenColors (tested through resolveTheme with includes)", () => {
  it("includes base tokenColors when no overlay matches", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [
        { scope: "comment", settings: { foreground: "#111" } },
        { scope: "string", settings: { foreground: "#222" } },
      ],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.tokenColors).toHaveLength(2);
  });

  it("appends new scope from overlay", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [
        { scope: "comment", settings: { foreground: "#111" } },
        { scope: "keyword", settings: { foreground: "#333" } },
      ],
    };
    const result = resolveTheme(darkEntry, json);
    const scopes = result.tokenColors.map((tc) => tc.scope);
    expect(scopes).toContain("comment");
    expect(scopes).toContain("keyword");
  });

  it("preserves tokenColor order", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [
        { scope: "comment", settings: { foreground: "#AAA" } },
        { scope: "string", settings: { foreground: "#BBB" } },
        { scope: "keyword", settings: { foreground: "#CCC" } },
      ],
    };
    const result = resolveTheme(darkEntry, json);
    expect(result.tokenColors[0].scope).toBe("comment");
    expect(result.tokenColors[1].scope).toBe("string");
    expect(result.tokenColors[2].scope).toBe("keyword");
  });

  it("merges settings when scopes match", () => {
    const json: VscodeThemeJson = {
      colors: {},
      tokenColors: [
        { scope: "comment", settings: { foreground: "#111" } },
        { scope: "comment", settings: { fontStyle: "italic" } },
      ],
    };
    const result = resolveTheme(darkEntry, json);
    const commentTokens = result.tokenColors.filter((tc) => tc.scope === "comment");
    expect(commentTokens).toHaveLength(1);
    expect(commentTokens[0].settings.foreground).toBe("#111");
    expect(commentTokens[0].settings.fontStyle).toBe("italic");
  });
});
