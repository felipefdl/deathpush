import { describe, expect, it } from "vite-plus/test";
import { pierreFileRenderInput } from "./file-render-input";

describe("pierreFileRenderInput", () => {
  it("renders as plain text when the language highlighter is not ready", () => {
    expect(pierreFileRenderInput("vite.config.ts", "export default {}", "k", false)).toEqual({
      name: "vite.config.ts",
      contents: "export default {}",
      cacheKey: "k",
      lang: "text",
    });
  });

  it("infers the language once the highlighter is ready", () => {
    expect(pierreFileRenderInput("vite.config.ts", "export default {}", "k", true)).toEqual({
      name: "vite.config.ts",
      contents: "export default {}",
      cacheKey: "k",
    });
  });
});
