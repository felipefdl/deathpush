import { getOrCreateWorkerPoolSingleton, type WorkerPoolManager } from "@pierre/diffs/worker";
import PierreWorker from "@pierre/diffs/worker/worker.js?worker";

export const getPierreWorkerPool = (): WorkerPoolManager =>
  getOrCreateWorkerPoolSingleton({
    poolOptions: {
      workerFactory: () => new PierreWorker(),
    },
    highlighterOptions: {
      preferredHighlighter: "shiki-js",
      lineDiffType: "word-alt",
    },
  });

export const applyPierrePoolTheme = (themeId: string): void => {
  void getPierreWorkerPool().setRenderOptions({
    theme: themeId,
    lineDiffType: "word-alt",
  });
};
