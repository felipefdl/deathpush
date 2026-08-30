import { describe, it, expect } from "vite-plus/test";
import { buildPierreDiffOptions } from "./options";

const base = {
  themeId: "preview-theme",
  themeType: "dark" as const,
  wordWrap: "off" as const,
  diffMode: "inline" as const,
  enableLineSelection: true,
  showLineNumbers: false,
  diffIndicators: "bars" as const,
  lineDiffType: "char" as const,
  showBackground: false,
  hunkSeparators: "metadata" as const,
};

describe("buildPierreDiffOptions", () => {
  it("maps inline to unified and sideBySide to split", () => {
    expect(buildPierreDiffOptions(base).diffStyle).toBe("unified");
    expect(buildPierreDiffOptions({ ...base, diffMode: "sideBySide" }).diffStyle).toBe("split");
  });

  it("maps wrap on to wrap", () => {
    expect(buildPierreDiffOptions({ ...base, wordWrap: "on" }).overflow).toBe("wrap");
    expect(buildPierreDiffOptions(base).overflow).toBe("scroll");
  });

  it("hides Pierre's bottom-only native horizontal scrollbar", () => {
    expect(buildPierreDiffOptions(base).unsafeCSS).toContain("[data-code]::-webkit-scrollbar");
  });

  it("pins the selected theme type and shiki-js", () => {
    const options = buildPierreDiffOptions(base);
    expect(options.theme).toBe("preview-theme");
    expect(options.themeType).toBe("dark");
    expect(buildPierreDiffOptions({ ...base, themeType: "light" }).themeType).toBe("light");
    expect(options.preferredHighlighter).toBe("shiki-js");
    expect(options.enableLineSelection).toBe(true);
  });

  it("maps user-facing diff preferences", () => {
    const options = buildPierreDiffOptions(base);
    expect(options.disableLineNumbers).toBe(true);
    expect(options.diffIndicators).toBe("bars");
    expect(options.lineDiffType).toBe("char");
    expect(options.disableBackground).toBe(true);
    expect(options.hunkSeparators).toBe("metadata");
  });
});
