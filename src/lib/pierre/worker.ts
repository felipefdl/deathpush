import { getSharedHighlighter, type SupportedLanguages } from "@pierre/diffs";
import { getOrCreateWorkerPoolSingleton, type WorkerPoolManager } from "@pierre/diffs/worker";
import PierreWorker from "@pierre/diffs/worker/worker.js?worker";

export const PIERRE_WARM_LANGUAGES: SupportedLanguages[] = ["typescript", "javascript", "json"];

let languageWarmupScheduled = false;

export const warmPierreHighlighter = (themeId: string): void => {
  if (languageWarmupScheduled) return;
  languageWarmupScheduled = true;
  void getSharedHighlighter({
    langs: PIERRE_WARM_LANGUAGES,
    themes: [themeId],
    preferredHighlighter: "shiki-js",
  }).then((highlighter) => {
    highlighter.codeToHast("export const n: number = 1;\n", { lang: "typescript", theme: themeId });
  });
};

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
  warmPierreHighlighter(themeId);
};
