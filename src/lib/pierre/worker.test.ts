import { afterEach, describe, expect, it, vi } from "vite-plus/test";

const { getSharedHighlighterMock, codeToHastMock } = vi.hoisted(() => {
  const codeToHastMock = vi.fn();
  return {
    codeToHastMock,
    getSharedHighlighterMock: vi.fn(async () => ({ codeToHast: codeToHastMock })),
  };
});

vi.mock("@pierre/diffs", () => ({
  getSharedHighlighter: getSharedHighlighterMock,
}));

vi.mock("@pierre/diffs/worker", () => ({
  getOrCreateWorkerPoolSingleton: () => ({
    setRenderOptions: vi.fn(async () => undefined),
  }),
}));

vi.mock("@pierre/diffs/worker/worker.js?worker", () => ({
  default: class {},
}));

import { applyPierrePoolTheme, PIERRE_WARM_LANGUAGES } from "./worker";

describe("Pierre highlighter warmup", () => {
  afterEach(() => {
    getSharedHighlighterMock.mockClear();
    codeToHastMock.mockClear();
  });

  it("includes TypeScript so explorer files do not wait on first highlight", () => {
    expect(PIERRE_WARM_LANGUAGES).toContain("typescript");
  });

  it("tokenizes TypeScript immediately after a theme is applied", async () => {
    applyPierrePoolTheme("vesper");
    await Promise.resolve();

    expect(getSharedHighlighterMock).toHaveBeenCalledWith({
      langs: PIERRE_WARM_LANGUAGES,
      themes: ["vesper"],
      preferredHighlighter: "shiki-js",
    });
    expect(codeToHastMock).toHaveBeenCalledWith("export const n: number = 1;\n", {
      lang: "typescript",
      theme: "vesper",
    });
  });
});
