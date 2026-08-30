import { describe, it, expect } from "vite-plus/test";
import { normalizeWordWrap, pierreHostStyle } from "./normalize-editor-settings";

describe("normalizeWordWrap", () => {
  it("keeps off", () => {
    expect(normalizeWordWrap("off")).toBe("off");
  });

  it("keeps on", () => {
    expect(normalizeWordWrap("on")).toBe("on");
  });

  it("treats undefined as on", () => {
    expect(normalizeWordWrap(undefined)).toBe("on");
  });

  it("maps wordWrapColumn and bounded to on", () => {
    expect(normalizeWordWrap("wordWrapColumn")).toBe("on");
    expect(normalizeWordWrap("bounded")).toBe("on");
  });
});

describe("Pierre editable layout", () => {
  it("removes the code padding that offsets Pierre's caret", () => {
    expect(
      pierreHostStyle({
        fontFamily: "Menlo",
        fontSize: 14,
        lineHeight: 22,
        tabSize: 2,
      })["--diffs-gap-block"]
    ).toBe("0px");
  });
});

describe("pierreHostStyle", () => {
  it("uses the active editor background across the full host", () => {
    expect(
      pierreHostStyle({
        fontFamily: "Menlo",
        fontSize: 14,
        lineHeight: 22,
        tabSize: 2,
      })["background-color"]
    ).toBe("var(--vscode-editor-background)");
  });

  it("maps editor metrics and sizing onto the host", () => {
    expect(
      pierreHostStyle({
        fontFamily: "Menlo",
        fontSize: 14,
        lineHeight: 22,
        tabSize: 2,
      })
    ).toEqual({
      width: "100%",
      height: "100%",
      "min-width": "0",
      "overflow-x": "hidden",
      "overflow-y": "auto",
      display: "flex",
      "flex-direction": "column",
      "background-color": "var(--vscode-editor-background)",
      "--diffs-gap-block": "0px",
      "--diffs-font-family": "Menlo",
      "--diffs-font-size": "14px",
      "--diffs-line-height": "22px",
      "--diffs-tab-size": 2,
      "--diffs-light-bg": "var(--vscode-editor-background)",
      "--diffs-dark-bg": "var(--vscode-editor-background)",
      "--diffs-light": "var(--vscode-editor-foreground)",
      "--diffs-dark": "var(--vscode-editor-foreground)",
      "font-family": "Menlo",
      "font-size": "14px",
      "line-height": "22px",
      "tab-size": 2,
    });
  });
});
