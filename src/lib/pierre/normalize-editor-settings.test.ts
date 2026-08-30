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

describe("pierreHostStyle", () => {
  it("maps font family, size, line height, and tab size onto the host", () => {
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
      overflow: "auto",
      "font-family": "Menlo",
      "font-size": "14px",
      "line-height": "22px",
      "tab-size": 2,
    });
  });
});
