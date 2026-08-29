import { describe, it, expect } from "vite-plus/test";
import { normalizeWordWrap } from "./normalize-editor-settings";

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
