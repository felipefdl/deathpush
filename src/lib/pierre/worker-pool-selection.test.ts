import { afterEach, describe, expect, it } from "vite-plus/test";
import { WorkerPoolManager } from "@pierre/diffs/worker";

const FILE_OPTIONS = {
  theme: { dark: "pierre-dark", light: "pierre-light" },
  useTokenTransformer: false,
  tokenizeMaxLineLength: 1000,
};

const FILE_RESULT = { code: [], themeStyles: "", baseThemeType: "dark" as const };

class FakeWorker {
  received: string[] = [];
  pendingReply: (() => void) | undefined;
  holdWarmup = false;
  private readonly listeners = new Map<string, Array<(event: { data: unknown }) => void>>();

  addEventListener(type: string, listener: (event: { data: unknown }) => void): void {
    const list = this.listeners.get(type) ?? [];
    list.push(listener);
    this.listeners.set(type, list);
  }

  terminate(): void {}

  postMessage(request: { type: string; id: string; file?: { name: string } }): void {
    if (request.type === "initialize") {
      this.emit({ type: "success", requestType: "initialize", id: request.id, sentAt: 0 });
      return;
    }
    if (request.type !== "file") return;
    if (request.file === undefined) return;
    this.received.push(request.file.name);
    const reply = (): void => {
      this.emit({
        type: "success",
        requestType: "file",
        id: request.id,
        result: FILE_RESULT,
        options: FILE_OPTIONS,
        sentAt: 0,
      });
      this.pendingReply = undefined;
    };
    if (this.holdWarmup && request.file.name.startsWith("warmup")) {
      this.pendingReply = reply;
      return;
    }
    queueMicrotask(reply);
  }

  private emit(data: unknown): void {
    for (const listener of this.listeners.get("message") ?? []) listener({ data });
  }
}

describe("Pierre 1.3.6 idle-worker selection", () => {
  let pool: WorkerPoolManager | undefined;
  let workers: FakeWorker[] = [];

  afterEach(() => {
    for (const worker of workers) worker.pendingReply?.();
    pool?.terminate();
    pool = undefined;
    workers = [];
  });

  const createPool = (poolSize: number, holdWarmup = false): WorkerPoolManager => {
    workers = [];
    pool = new WorkerPoolManager(
      {
        poolSize,
        workerFactory: () => {
          const worker = new FakeWorker();
          worker.holdWarmup = holdWarmup;
          workers.push(worker);
          return worker as unknown as Worker;
        },
      },
      { preferredHighlighter: "shiki-js", lineDiffType: "word-alt" }
    );
    return pool;
  };

  it("keeps sequential TS then TSX primes on the last worker", async () => {
    const manager = createPool(2);
    await manager.initialize();

    await manager.primeFileHighlightCache({
      name: "warmup.ts",
      contents: "export {};\n",
      cacheKey: "pierre-warmup:ts",
    });
    await manager.primeFileHighlightCache({
      name: "warmup.tsx",
      contents: "export const n = <i />;\n",
      cacheKey: "pierre-warmup:tsx",
    });

    expect(workers.map((worker) => worker.received)).toEqual([[], ["warmup.ts", "warmup.tsx"]]);
  });
  it("lets a rust highlight finish while a TS prime occupies the last worker", async () => {
    const manager = createPool(2, true);
    await manager.initialize();

    const tsPrime = manager.primeFileHighlightCache({
      name: "warmup.ts",
      contents: "export {};\n",
      cacheKey: "pierre-warmup:ts",
    });

    await expect.poll(() => workers.some((worker) => worker.pendingReply !== null)).toBe(true);
    expect(manager.getStats().busyWorkers).toBe(1);

    let rustSettled = false;
    void manager
      .primeFileHighlightCache({
        name: "main.rs",
        contents: "fn main() {}\n",
        cacheKey: "rust-foreground",
      })
      .then(() => {
        rustSettled = true;
      });

    await expect.poll(() => rustSettled).toBe(true);
    expect(workers.some((worker) => worker.pendingReply !== null)).toBe(true);
    expect(manager.getStats().busyWorkers).toBe(1);

    workers.find((worker) => worker.pendingReply !== null)?.pendingReply?.();
    await tsPrime;
  });
});
