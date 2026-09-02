import { getFiletypeFromFileName } from "@pierre/diffs";
import { getOrCreateWorkerPoolSingleton, type WorkerPoolManager } from "@pierre/diffs/worker";
import PierreWorker from "@pierre/diffs/worker/worker.js?worker";

const PIERRE_POOL_SIZE = 2;

const TS_PRIME = {
  name: "warmup.ts",
  contents:
    'import { x } from "./x";\nexport const n = 1;\nexport function f<T>(v: T): T { return v; }\ntype A = { a: string; b?: number };\ninterface B { m(): void }\nclass C implements B { m() { return `${n}`; } }\nconst arrow = (a: A) => a.a;\n',
  cacheKey: "pierre-warmup:ts",
};
const TSX_PRIME = {
  name: "warmup.tsx",
  contents:
    'import { x } from "./x";\nexport const n = <div className="x">{1}</div>;\nexport function F({ a }: { a: string }) { return <span>{a}</span>; }\nconst frag = <>{n}</>;\n',
  cacheKey: "pierre-warmup:tsx",
};

let poolIdleStarted = false;
let tsPrimeStarted = false;

export const getPierreWorkerPool = (): WorkerPoolManager =>
  getOrCreateWorkerPoolSingleton({
    poolOptions: {
      workerFactory: () => new PierreWorker(),
      poolSize: PIERRE_POOL_SIZE,
    },
    highlighterOptions: {
      preferredHighlighter: "shiki-js",
      lineDiffType: "word-alt",
    },
  });

export const pathsNeedPierreTsPrime = (paths: readonly string[]): boolean => {
  for (const path of paths) {
    const lang = getFiletypeFromFileName(path);
    if (lang === "typescript" || lang === "tsx") return true;
  }
  return false;
};

export const warmPierreWorkerPool = async (): Promise<void> => {
  const pool = getPierreWorkerPool();
  await pool.initialize();
  await pool.primeFileHighlightCache(TS_PRIME);
  await pool.primeFileHighlightCache(TSX_PRIME);
};

const runIdle = (work: () => void): void => {
  if (typeof requestIdleCallback === "function") {
    requestIdleCallback(work);
    return;
  }
  setTimeout(work, 0);
};

export const schedulePierreWorkerWarmup = (paths: readonly string[] = []): void => {
  if (!poolIdleStarted) {
    poolIdleStarted = true;
    runIdle(() => {
      void getPierreWorkerPool()
        .initialize()
        .catch(() => {
          poolIdleStarted = false;
        });
    });
  }
  if (tsPrimeStarted || !pathsNeedPierreTsPrime(paths)) return;
  tsPrimeStarted = true;
  runIdle(() => {
    void warmPierreWorkerPool().catch(() => {
      tsPrimeStarted = false;
    });
  });
};
