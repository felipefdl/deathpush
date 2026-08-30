import { describe, it, expect, vi } from "vite-plus/test";
import { flushAll, flushPath, flushPaths, registerFlusher, trackPendingFlush } from "./flush-registry";

describe("flush-registry", () => {
  it("flushPath runs the flusher registered for that path", async () => {
    const flush = vi.fn(async () => undefined);
    const unregister = registerFlusher("src/a.ts", flush);

    await flushPath("src/a.ts");
    expect(flush).toHaveBeenCalledTimes(1);

    unregister();
  });

  it("flushAll runs every registered flusher", async () => {
    const first = vi.fn(async () => undefined);
    const second = vi.fn(async () => undefined);
    const dropFirst = registerFlusher("src/a.ts", first);
    const dropSecond = registerFlusher("src/b.ts", second);

    await flushAll();
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);

    dropFirst();
    dropSecond();
  });

  it("unregister only deletes the flush function it registered", async () => {
    const stale = vi.fn(async () => undefined);
    const current = vi.fn(async () => undefined);
    const dropStale = registerFlusher("src/a.ts", stale);
    registerFlusher("src/a.ts", current);
    dropStale();

    await flushPath("src/a.ts");
    expect(stale).not.toHaveBeenCalled();
    expect(current).toHaveBeenCalledTimes(1);
  });

  it("flushPath runs every flusher registered for the same path", async () => {
    const first = vi.fn(async () => undefined);
    const second = vi.fn(async () => undefined);
    const dropFirst = registerFlusher("src/a.ts", first);
    const dropSecond = registerFlusher("src/a.ts", second);

    await flushPath("src/a.ts");
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);

    dropFirst();
    dropSecond();
  });

  it("unregistering one instance keeps the other path flusher", async () => {
    const first = vi.fn(async () => undefined);
    const second = vi.fn(async () => undefined);
    const dropFirst = registerFlusher("src/a.ts", first);
    const dropSecond = registerFlusher("src/a.ts", second);
    dropFirst();

    await flushPath("src/a.ts");
    expect(first).not.toHaveBeenCalled();
    expect(second).toHaveBeenCalledTimes(1);

    dropSecond();
  });

  it("flushPath awaits a cleanup write after unregister", async () => {
    let resolveFlush: (() => void) | undefined;
    const flush = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveFlush = resolve;
        })
    );
    const unregister = registerFlusher("src/a.ts", flush);
    void trackPendingFlush("src/a.ts", flush());
    unregister();

    let flushPathDone = false;
    const pending = flushPath("src/a.ts").then(() => {
      flushPathDone = true;
    });

    await Promise.resolve();
    expect(flushPathDone).toBe(false);

    resolveFlush?.();
    await pending;
    expect(flushPathDone).toBe(true);
  });

  it("flushPath is a no-op when the path has no flusher", async () => {
    await expect(flushPath("missing.ts")).resolves.toBeUndefined();
  });

  it("flushAll awaits a cleanup flush after unregister", async () => {
    let resolveFlush: (() => void) | undefined;
    const flush = vi.fn(
      () =>
        new Promise<void>((resolve) => {
          resolveFlush = resolve;
        })
    );
    const unregister = registerFlusher("src/a.ts", flush);
    void trackPendingFlush("src/a.ts", flush());
    unregister();

    let flushAllDone = false;
    const all = flushAll().then(() => {
      flushAllDone = true;
    });

    await Promise.resolve();
    expect(flushAllDone).toBe(false);

    resolveFlush?.();
    await all;
    expect(flushAllDone).toBe(true);
  });

  it("flushPaths runs each registered flusher", async () => {
    const first = vi.fn(async () => undefined);
    const second = vi.fn(async () => undefined);
    const dropFirst = registerFlusher("src/a.ts", first);
    const dropSecond = registerFlusher("src/b.ts", second);

    await flushPaths(["src/a.ts", "src/b.ts", "missing.ts"]);
    expect(first).toHaveBeenCalledTimes(1);
    expect(second).toHaveBeenCalledTimes(1);

    dropFirst();
    dropSecond();
  });
});
