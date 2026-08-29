import { describe, it, expect, vi } from "vite-plus/test";
import { flushAll, flushPath, registerFlusher } from "./flush-registry";

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

  it("flushPath is a no-op when the path has no flusher", async () => {
    await expect(flushPath("missing.ts")).resolves.toBeUndefined();
  });
});
