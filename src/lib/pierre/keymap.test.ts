import { describe, it, expect } from "vite-plus/test";
import { pierreEditorKeymap } from "./keymap";

describe("pierreEditorKeymap", () => {
  it("overrides ctrl+k only on mac so Windows and Linux keep default bindings", () => {
    expect(pierreEditorKeymap).toEqual([
      {
        platform: "mac",
        bindings: {
          "ctrl+k": "simplifySelection",
        },
      },
    ]);
  });
});
