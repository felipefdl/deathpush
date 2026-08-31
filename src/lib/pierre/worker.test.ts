import { beforeEach, describe, expect, it, vi } from "vite-plus/test";

const { getOrCreateWorkerPoolSingletonMock } = vi.hoisted(() => ({
  getOrCreateWorkerPoolSingletonMock: vi.fn(() => ({ setRenderOptions: vi.fn() })),
}));

vi.mock("@pierre/diffs/worker", () => ({
  getOrCreateWorkerPoolSingleton: getOrCreateWorkerPoolSingletonMock,
}));

vi.mock("@pierre/diffs/worker/worker.js?worker", () => ({
  default: class {},
}));

import { getPierreWorkerPool } from "./worker";

describe("getPierreWorkerPool", () => {
  beforeEach(() => {
    getOrCreateWorkerPoolSingletonMock.mockClear();
  });

  it("creates the shared worker pool only when requested", () => {
    getPierreWorkerPool();

    expect(getOrCreateWorkerPoolSingletonMock).toHaveBeenCalledOnce();
    expect(getOrCreateWorkerPoolSingletonMock).toHaveBeenCalledWith({
      poolOptions: {
        workerFactory: expect.any(Function),
      },
      highlighterOptions: {
        preferredHighlighter: "shiki-js",
        lineDiffType: "word-alt",
      },
    });
  });
});
