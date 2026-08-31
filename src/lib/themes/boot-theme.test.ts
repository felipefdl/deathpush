import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { getBootTheme } from "./boot-theme";

describe("getBootTheme", () => {
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it("uses the light bundled theme when the system prefers light", () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({ matches: false }))
    );

    const theme = getBootTheme();

    expect(theme.id).toBe("ayu-light");
    expect(theme.kind).toBe("light");
  });

  it("uses the dark bundled theme when the system prefers dark", () => {
    vi.stubGlobal(
      "matchMedia",
      vi.fn(() => ({ matches: true }))
    );

    const theme = getBootTheme();

    expect(theme.id).toBe("vesper");
    expect(theme.kind).toBe("dark");
  });
});
