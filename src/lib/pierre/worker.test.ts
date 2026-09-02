import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";

const { getOrCreateWorkerPoolSingletonMock, pool } = vi.hoisted(() => {
  const pool = {
    initialize: vi.fn(async () => undefined),
    primeFileHighlightCache: vi.fn(async () => undefined),
    setRenderOptions: vi.fn(),
  };
  return {
    pool,
    getOrCreateWorkerPoolSingletonMock: vi.fn(() => pool),
  };
});

vi.mock("@pierre/diffs/worker", () => ({
  getOrCreateWorkerPoolSingleton: getOrCreateWorkerPoolSingletonMock,
}));

vi.mock("@pierre/diffs/worker/worker.js?worker", () => ({
  default: class {},
}));

import { getPierreWorkerPool, pathsNeedPierreTsPrime, warmPierreWorkerPool } from "./worker";

const importWorker = async () => {
  vi.resetModules();
  return import("./worker");
};

const resetPoolMocks = (): void => {
  getOrCreateWorkerPoolSingletonMock.mockClear();
  pool.initialize.mockReset();
  pool.primeFileHighlightCache.mockReset();
  pool.initialize.mockResolvedValue(undefined);
  pool.primeFileHighlightCache.mockResolvedValue(undefined);
};

const stubIdleCallback = (): void => {
  vi.stubGlobal("requestIdleCallback", (cb: () => void) => {
    cb();
    return 1;
  });
};

describe("getPierreWorkerPool", () => {
  beforeEach(() => {
    resetPoolMocks();
  });

  it("creates the shared worker pool only when requested", () => {
    getPierreWorkerPool();

    expect(getOrCreateWorkerPoolSingletonMock).toHaveBeenCalledOnce();
    expect(getOrCreateWorkerPoolSingletonMock).toHaveBeenCalledWith({
      poolOptions: {
        workerFactory: expect.any(Function),
        poolSize: 2,
      },
      highlighterOptions: {
        preferredHighlighter: "shiki-js",
        lineDiffType: "word-alt",
      },
    });
    expect(getOrCreateWorkerPoolSingletonMock.mock.calls[0]?.[0].highlighterOptions.langs).toBeUndefined();
  });
});

describe("pathsNeedPierreTsPrime", () => {
  it("detects typescript and tsx paths and ignores rust", () => {
    expect(pathsNeedPierreTsPrime(["src/main.rs"])).toBe(false);
    expect(pathsNeedPierreTsPrime(["src/lib/worker.ts"])).toBe(true);
    expect(pathsNeedPierreTsPrime(["src/app.tsx"])).toBe(true);
    expect(pathsNeedPierreTsPrime(["src/mod.mts", "src/main.rs"])).toBe(true);
  });
});

describe("warmPierreWorkerPool", () => {
  beforeEach(() => {
    resetPoolMocks();
  });

  it("avoids transferring typescript and tsx grammars on a later click", async () => {
    const primed = new Set<string>();
    pool.primeFileHighlightCache.mockImplementation(async (file: { name: string }) => {
      if (file.name.endsWith(".tsx")) primed.add("tsx");
      else if (file.name.endsWith(".ts")) primed.add("typescript");
    });

    expect(primed.has("typescript")).toBe(false);
    expect(primed.has("tsx")).toBe(false);

    await warmPierreWorkerPool();

    expect(pool.initialize).toHaveBeenCalledOnce();
    expect(primed.has("typescript")).toBe(true);
    expect(primed.has("tsx")).toBe(true);
    expect(pool.primeFileHighlightCache.mock.calls.map((call) => call[0].name)).toEqual(["warmup.ts", "warmup.tsx"]);
  });
});

describe("schedulePierreWorkerWarmup", () => {
  beforeEach(() => {
    resetPoolMocks();
    stubIdleCallback();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("idle-creates the singleton without langs and primes ts files when present", async () => {
    const { schedulePierreWorkerWarmup } = await importWorker();

    schedulePierreWorkerWarmup(["src/main.rs"]);
    await Promise.resolve();

    expect(getOrCreateWorkerPoolSingletonMock).toHaveBeenCalledOnce();
    expect(pool.initialize).toHaveBeenCalledOnce();
    expect(pool.primeFileHighlightCache).not.toHaveBeenCalled();

    schedulePierreWorkerWarmup(["src/app.tsx"]);
    await Promise.resolve();
    await Promise.resolve();

    expect(pool.primeFileHighlightCache).toHaveBeenCalled();
    expect(pool.primeFileHighlightCache.mock.calls.map((call) => call[0].name)).toEqual(["warmup.ts", "warmup.tsx"]);
  });

  it("retries pool initialize and ts prime after a background rejection", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("requestIdleCallback", (cb: () => void) => {
      setTimeout(cb, 0);
      return 1;
    });
    const { schedulePierreWorkerWarmup } = await importWorker();

    pool.initialize.mockRejectedValueOnce(new Error("init failed"));
    schedulePierreWorkerWarmup(["src/main.rs"]);
    await vi.runAllTimersAsync();

    expect(pool.initialize).toHaveBeenCalledOnce();

    schedulePierreWorkerWarmup(["src/main.rs"]);
    await vi.runAllTimersAsync();

    expect(pool.initialize).toHaveBeenCalledTimes(2);

    pool.primeFileHighlightCache.mockRejectedValueOnce(new Error("prime failed"));
    schedulePierreWorkerWarmup(["src/app.tsx"]);
    await vi.runAllTimersAsync();

    expect(pool.primeFileHighlightCache).toHaveBeenCalled();
    const afterFail = pool.primeFileHighlightCache.mock.calls.length;

    schedulePierreWorkerWarmup(["src/app.tsx"]);
    await vi.runAllTimersAsync();

    expect(pool.primeFileHighlightCache.mock.calls.length).toBeGreaterThan(afterFail);
  });

  it("falls back to setTimeout so warmup does not run in the current render microtask", async () => {
    vi.unstubAllGlobals();
    vi.stubGlobal("requestIdleCallback", undefined);
    vi.useFakeTimers();

    const { schedulePierreWorkerWarmup } = await importWorker();
    schedulePierreWorkerWarmup(["src/main.rs"]);
    await Promise.resolve();

    expect(pool.initialize).not.toHaveBeenCalled();

    await vi.runAllTimersAsync();

    expect(pool.initialize).toHaveBeenCalledOnce();
  });
});
