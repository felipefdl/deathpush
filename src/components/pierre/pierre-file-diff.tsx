import { createEffect, createSignal, onSettled } from "solid-js";
import {
  FileDiff,
  parseDiffFromFile,
  parsePatchFiles,
  type DiffLineAnnotation,
  type FileContents,
  type FileDiffMetadata,
  type PostRenderPhase,
  type SelectedLineRange,
} from "@pierre/diffs";
import { Editor } from "@pierre/diffs/edit";
import type { DiffHunkPayload, DiffPayload, ResourceGroupKind } from "../../lib/git-types";

import * as commands from "../../lib/tauri-commands";
import { sendDestructiveIntent, sendIntent } from "../../lib/session-client";
import { takeScmDiffPayload } from "../../lib/pierre/scm-diff-payload";

import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { buildPierreDiffOptions } from "../../lib/pierre/options";

import { normalizeWordWrap, pierreHostStyle } from "../../lib/pierre/normalize-editor-settings";
import { pierreEditorKeymap } from "../../lib/pierre/keymap";
import { getPierreWorkerPool } from "../../lib/pierre/worker";
import { flushPath, registerFlusher, trackPendingFlush } from "../../lib/pierre/flush-registry";
import { commitPierreWrite } from "../../lib/pierre/buffered-write";
import { hunkActionAnchor } from "../../lib/pierre/hunk-annotations";
import { normalizeSelectionRange } from "../../lib/pierre/line-map";

import { isDirty, sessionCacheKey, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";
import { createPierreFindHost, type PierreFindHost } from "../../lib/pierre/find-host";
import { registerScmSession } from "../../lib/pierre/scm-session-registry";
import { selectionIsInPierreHost } from "./pierre-file";
import { PierreScrollHost, type PierreScrollHostHandle } from "./pierre-scroll-host";

const SAVE_MS = 1000;

export type HunkAnnotationMeta = { hunkId: string };

export type PierreScmDiffProps = {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
  loadId: number;
};

export type PierreHistoryDiffProps = {
  path: string;
  original: string;
  modified: string;
  cacheKey: string;
};

export type PierreFileDiffProps = PierreScmDiffProps | PierreHistoryDiffProps;

const pierreShadowRoot = (content: HTMLElement): ShadowRoot | undefined =>
  content.querySelector("diffs-container")?.shadowRoot ?? undefined;

const readOnlyBlame = (setCursorLine: (lineNumber: number) => void) => ({
  onLineClick: (props: { lineNumber: number }) => {
    setCursorLine(props.lineNumber);
  },
  onLineSelected: (range: SelectedLineRange | null) => {
    if (range) setCursorLine(range.end);
  },
});

const refreshFindAfterRender =
  (findHost: PierreFindHost | undefined) =>
  (_node: HTMLElement, _instance: FileDiff, phase: PostRenderPhase): void => {
    if (phase !== "unmount") findHost?.refresh();
  };

const splitHistoryLines = (contents: string): string[] => (contents === "" ? [] : contents.split(/(?<=\n)/));

export const historyCacheKey = (commitId: string, path: string): string => `${commitId}:${path}`;

export const historyFileDiff = (
  path: string,
  original: string,
  modified: string,
  cacheKey: string
): FileDiffMetadata => {
  if (original !== modified) {
    return {
      ...parseDiffFromFile({ name: path, contents: original, cacheKey }, { name: path, contents: modified, cacheKey }),
      cacheKey,
    };
  }
  const lines = splitHistoryLines(original);
  const count = lines.length;
  if (count === 0) {
    return {
      name: path,
      type: "change",
      hunks: [],
      splitLineCount: 0,
      unifiedLineCount: 0,
      isPartial: false,
      additionLines: [],
      deletionLines: [],
      cacheKey,
    };
  }
  const noEof = !original.endsWith("\n");
  return {
    name: path,
    type: "change",
    hunks: [
      {
        collapsedBefore: 0,
        splitLineCount: count,
        splitLineStart: 0,
        unifiedLineCount: count,
        unifiedLineStart: 0,
        additionCount: count,
        additionStart: 1,
        additionLines: 0,
        deletionCount: count,
        deletionStart: 1,
        deletionLines: 0,
        deletionLineIndex: 0,
        additionLineIndex: 0,
        hunkContent: [{ type: "context", lines: count, additionLineIndex: 0, deletionLineIndex: 0 }],
        hunkSpecs: `@@ -1,${count} +1,${count} @@\n`,
        noEOFCRAdditions: noEof,
        noEOFCRDeletions: noEof,
      },
    ],
    splitLineCount: count,
    unifiedLineCount: count,
    isPartial: false,
    additionLines: lines,
    deletionLines: lines,
    cacheKey,
  };
};

export const isNonPierreFileType = (fileType: string): boolean =>
  fileType === "image" || fileType === "binary" || fileType === "large";

export const loadScmDiffSources = async (input: {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
  loadId: number;
  consumeCache?: boolean;
}): Promise<DiffPayload> => {
  if (input.consumeCache) {
    const cached = takeScmDiffPayload({
      path: input.path,
      staged: input.staged,
      groupKind: input.groupKind,
      loadId: input.loadId,
    });
    if (cached) return cached;
  }
  const result = await sendIntent({
    type: "openScmDiff",
    path: input.path,
    staged: input.staged,
    groupKind: input.groupKind,
  });
  if (result.kind !== "diff") {
    throw new Error("Expected a diff payload");
  }
  return result.payload;
};

export type EmptyPatchSides =
  | { oldFile: null; newFile: FileContents }
  | { oldFile: FileContents; newFile: null }
  | { oldFile: FileContents; newFile: FileContents };

export const emptyPatchSides = (
  path: string,
  cacheKey: string,
  original: string,
  modified: string,
  presence: { oldExists: boolean; newExists: boolean }
): EmptyPatchSides => {
  if (!presence.oldExists) {
    return { oldFile: null, newFile: { name: path, contents: modified, cacheKey } };
  }
  if (!presence.newExists) {
    return { oldFile: { name: path, contents: original, cacheKey }, newFile: null };
  }
  return {
    oldFile: { name: path, contents: original, cacheKey },
    newFile: { name: path, contents: modified, cacheKey },
  };
};

export const hunkAnnotations = (hunks: DiffHunkPayload[]): DiffLineAnnotation<HunkAnnotationMeta>[] => {
  const annotations: DiffLineAnnotation<HunkAnnotationMeta>[] = [];
  for (const hunk of hunks) {
    const anchor = hunkActionAnchor(hunk);
    if (!anchor) continue;
    annotations.push({
      side: anchor.side,
      lineNumber: anchor.lineNumber,
      metadata: { hunkId: hunk.id },
    });
  }
  return annotations;
};

const hunkButton = (label: string, onClick: () => void): HTMLButtonElement => {
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = label;
  button.style.cssText = [
    "display:inline-flex",
    "align-items:center",
    "height:18px",
    "padding:0 6px",
    "font-size:11px",
    "border:1px solid var(--vscode-button-border, var(--vscode-panel-border))",
    "background:var(--vscode-button-secondaryBackground)",
    "color:var(--vscode-button-secondaryForeground)",
    "cursor:pointer",
    "border-radius:var(--radius-sm)",
    "font-family:var(--vscode-font-family)",
  ].join(";");
  button.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return button;
};

const renderHunkButtons = (
  hunkId: string,
  staged: boolean,
  run: (hunkId: string, action: "stage" | "unstage" | "discard") => void
): HTMLElement | undefined => {
  const row = document.createElement("div");
  row.style.cssText = "display:inline-flex;align-items:center;gap:4px;";
  if (staged) {
    row.append(hunkButton("Unstage", () => run(hunkId, "unstage")));
    return row;
  }
  row.append(
    hunkButton("Stage", () => run(hunkId, "stage")),
    hunkButton("Discard", () => run(hunkId, "discard"))
  );
  return row;
};

const PierreScmFileDiff = (props: PierreScmDiffProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const diffSettings = useStore(settingsStore, (s) => s.settings.diff);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [ready, setReady] = createSignal(false);
  const [cacheGeneration, setCacheGeneration] = createSignal(0);
  const [viewGeneration, setViewGeneration] = createSignal(0);
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let session: SaveSession | null = null;
  let fileRef: FileDiff | undefined;
  let editorRef: Editor<undefined> | undefined;
  let disposeEditRef: (() => void) | undefined;
  let activeSchedule: ((text: string) => void) | undefined;
  let scrollHost: PierreScrollHostHandle | undefined;
  let lineSelectionEnabled = false;

  onSettled(() => {
    setReady(true);
    return () => {
      setReady(false);
      activeSchedule = undefined;
      disposeEditRef?.();
      disposeEditRef = undefined;
      editorRef?.cleanUp();
      editorRef = undefined;
      fileRef?.cleanUp();
      fileRef = undefined;
    };
  });

  createEffect(
    () => [props.path, props.staged, props.groupKind] as const,
    ([path]) => {
      session = { path, diskSha: "", pendingSha: null, cacheGeneration: 0 };
      setCacheGeneration(0);
      setViewGeneration(0);
      repositoryStore.getState().setIsDiffDirty(false);
      return registerScmSession({
        session,
        reload: (diff, incomingSha) => {
          if (!session || session.path !== path) return;
          session.diskSha = incomingSha;
          session.pendingSha = null;
          session.cacheGeneration += 1;
          setCacheGeneration(session.cacheGeneration);
          repositoryStore.getState().setDiff(diff);
          repositoryStore.getState().setIsDiffDirty(false);
        },
      });
    }
  );

  createEffect(
    () =>
      ready()
        ? ([props.path, props.staged, props.groupKind, props.loadId, cacheGeneration(), viewGeneration()] as const)
        : null,
    (deps) => {
      if (!deps || !session) return;

      const [path, staged, groupKind, loadId, cacheGen, viewGen] = deps;
      const mountedGeneration = session.cacheGeneration;
      const activeSession = session;
      const { setError, setIsDiffDirty, setCursorLine, setDiff } = repositoryStore.getState();
      const theme = themeStore.getState().currentTheme;
      const settings = settingsStore.getState().settings;
      const themeId = theme.id;
      const themeType = theme.type;
      const wordWrap = normalizeWordWrap(settings.editor.wordWrap);
      const mode = settings.diff.layout;

      let cancelled = false;
      let pendingTimer: ReturnType<typeof setTimeout> | null = null;
      const pending = { text: null as string | null };
      let writeTail: Promise<void> = Promise.resolve();
      let unregisterFlush: (() => void) | undefined;
      let findHost: PierreFindHost | undefined;
      let busy = false;

      const syncDirty = (): void => {
        setIsDiffDirty(isDirty({ pendingTimer: pendingTimer !== null, pendingSha: activeSession.pendingSha }));
      };

      const writeOnce = (text: string): Promise<void> => {
        const work = async (): Promise<void> => {
          try {
            await commitPierreWrite({
              writeFile: () => commands.writeFile(path, text),
              pending,
              text,
              session: activeSession,
              sha256Utf8,
              syncDirty,
            });
          } catch (error) {
            setError(String(error));
            throw error;
          }
        };
        const run = writeTail.then(work, work);
        writeTail = run.then(
          () => undefined,
          () => undefined
        );
        return run;
      };

      const scheduleSave = (text: string): void => {
        pending.text = text;
        if (pendingTimer) clearTimeout(pendingTimer);
        pendingTimer = setTimeout(() => {
          pendingTimer = null;
          const next = pending.text;
          if (next === null) {
            syncDirty();
            return;
          }
          void writeOnce(next);
        }, SAVE_MS);
        syncDirty();
      };

      const flush = async (): Promise<void> => {
        if (pendingTimer) {
          clearTimeout(pendingTimer);
          pendingTimer = null;
        }
        if (pending.text !== null) {
          await writeOnce(pending.text);
          return;
        }
        await writeTail;
      };

      const afterGitWrite = (): void => {
        setViewGeneration((value) => value + 1);
      };

      const runHunkAction = async (hunkId: string, action: "stage" | "unstage" | "discard"): Promise<void> => {
        if (busy) return;
        busy = true;
        try {
          await flushPath(path);
          if (action === "discard") {
            await sendDestructiveIntent({ type: "discardHunk", hunkId, confirmed: false });
          } else if (action === "unstage") {
            await sendIntent({ type: "unstageHunk", hunkId });
          } else {
            await sendIntent({ type: "stageHunk", hunkId });
          }
          afterGitWrite();
        } catch (error) {
          setError(String(error));
        } finally {
          busy = false;
        }
      };

      let lineStaged = staged;
      const runLineSelection = async (range: SelectedLineRange): Promise<void> => {
        if (busy) return;
        busy = true;
        try {
          await flushPath(path);
          const normalized = normalizeSelectionRange(range);
          await sendIntent({
            type: "stageLines",
            path,
            start: normalized.start,
            end: normalized.end,
            staged: lineStaged,
          });
          afterGitWrite();
        } catch (error) {
          setError(String(error));
        } finally {
          busy = false;
        }
      };

      const onSelectionChange = (): void => {
        if (!editorRef) return;
        const node = document.getSelection()?.anchorNode;
        if (!node) return;
        if (!selectionIsInPierreHost(root, node)) return;
        const start = editorRef.getState().selections?.[0]?.start;
        if (start) setCursorLine(start.line + 1);
      };

      void (async () => {
        const payload = await loadScmDiffSources({
          path,
          staged,
          groupKind,
          loadId,
          consumeCache: cacheGen === 0 && viewGen === 0,
        });
        if (cancelled) return;
        lineStaged = payload.staged;
        lineSelectionEnabled = payload.enableLineSelection;
        if (isNonPierreFileType(payload.fileType)) return;

        if (activeSession.diskSha === "") {
          activeSession.diskSha = payload.contentHash;
        }
        if (cancelled) return;
        setDiff({
          path: payload.path,
          original: payload.original,
          modified: payload.modified,
          originalLanguage: payload.language,
          fileType: payload.fileType,
        });

        const cacheKey = sessionCacheKey(activeSession);
        const sides = emptyPatchSides(path, cacheKey, payload.original, payload.modified, payload.presence);
        const editable = payload.editable;
        const annotations = hunkAnnotations(payload.hunks);

        if (!editable) {
          findHost = createPierreFindHost({
            getRoot: () => pierreShadowRoot(content),
            wrapper: root,
          });
        }

        const options = {
          ...buildPierreDiffOptions({
            themeId,
            themeType,
            wordWrap,
            diffMode: mode,
            enableLineSelection: payload.enableLineSelection,
            ...settings.diff,
          }),
          renderAnnotation: (annotation: DiffLineAnnotation) => {
            if (!settingsStore.getState().settings.diff.showInlineHunkActions) return;
            const hunkId = annotations.find(
              (item) => item.side === annotation.side && item.lineNumber === annotation.lineNumber
            )?.metadata.hunkId;
            if (!hunkId) return;
            return renderHunkButtons(hunkId, payload.staged, (next, action) => {
              void runHunkAction(next, action);
            });
          },
          onLineSelectionEnd: (range: SelectedLineRange | null) => {
            if (!range) return;
            void runLineSelection(range);
          },
          ...(editable ? {} : readOnlyBlame(setCursorLine)),
          onPostRender: (node: HTMLElement, instance: FileDiff, phase: PostRenderPhase) => {
            refreshFindAfterRender(findHost)(node, instance, phase);
            if (phase !== "unmount") scrollHost?.finishRender();
          },
        };

        const file = fileRef ?? new FileDiff(options, getPierreWorkerPool());
        if (fileRef) {
          file.setOptions({ ...file.options, ...options });
        } else {
          fileRef = file;
        }
        if (!editable && disposeEditRef) {
          disposeEditRef();
          disposeEditRef = undefined;
          activeSchedule = undefined;
        }

        scrollHost?.beginRender();
        if (sides.newFile === null) {
          file.render({
            oldFile: sides.oldFile,
            newFile: null,
            lineAnnotations: annotations.map((item) => ({ side: item.side, lineNumber: item.lineNumber })),
            containerWrapper: content,
          });
        } else if (sides.oldFile === null) {
          file.render({
            oldFile: null,
            newFile: sides.newFile,
            lineAnnotations: annotations.map((item) => ({ side: item.side, lineNumber: item.lineNumber })),
            containerWrapper: content,
          });
        } else {
          file.render({
            oldFile: sides.oldFile,
            newFile: sides.newFile,
            lineAnnotations: annotations.map((item) => ({ side: item.side, lineNumber: item.lineNumber })),
            containerWrapper: content,
          });
        }
        scrollHost?.sync();

        if (editable) {
          unregisterFlush = registerFlusher(path, flush);
          activeSchedule = scheduleSave;
          editorRef ??= new Editor<undefined>({
            persistState: true,
            keymap: pierreEditorKeymap,
            onChange(next) {
              activeSchedule?.(next.contents);
            },
          });
          disposeEditRef ??= editorRef.edit(file);
          document.addEventListener("selectionchange", onSelectionChange);
        }
      })().catch((error: unknown) => {
        if (!cancelled) setError(String(error));
      });

      return () => {
        cancelled = true;
        document.removeEventListener("selectionchange", onSelectionChange);
        if (activeSession.cacheGeneration === mountedGeneration) {
          void trackPendingFlush(path, flush()).catch(() => undefined);
        } else if (pendingTimer) {
          clearTimeout(pendingTimer);
        }
        unregisterFlush?.();
        findHost?.dispose();
        if (activeSchedule === scheduleSave) activeSchedule = undefined;
      };
    }
  );

  createEffect(
    () =>
      [currentTheme().id, currentTheme().type, normalizeWordWrap(editorSettings().wordWrap), diffSettings()] as const,
    ([themeId, themeType, wordWrap, currentDiffSettings]) => {
      if (!fileRef) return;
      fileRef.setOptions({
        ...fileRef.options,
        ...buildPierreDiffOptions({
          themeId,
          themeType,
          wordWrap,
          diffMode: currentDiffSettings.layout,
          enableLineSelection: lineSelectionEnabled,
          ...currentDiffSettings,
        }),
      });
      fileRef.rerender();
    }
  );

  return (
    <PierreScrollHost
      style={pierreHostStyle(editorSettings())}
      rootRef={(element) => {
        root = element;
      }}
      contentRef={(element) => {
        content = element;
      }}
      handleRef={(handle) => {
        scrollHost = handle;
      }}
    />
  );
};

const PierreHistoryFileDiff = (props: PierreHistoryDiffProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const diffSettings = useStore(settingsStore, (s) => s.settings.diff);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [ready, setReady] = createSignal(false);
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let fileRef: FileDiff | undefined;
  let scrollHost: PierreScrollHostHandle | undefined;

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => (ready() ? ([props.path, props.original, props.modified, props.cacheKey] as const) : null),
    (deps) => {
      if (!deps) return;
      const [path, original, modified, cacheKey] = deps;
      const { setCursorLine, setError } = repositoryStore.getState();
      const theme = currentTheme();
      const themeId = theme.id;
      const themeType = theme.type;
      const wordWrap = normalizeWordWrap(editorSettings().wordWrap);
      const mode = diffSettings().layout;

      let findHost: PierreFindHost | undefined;
      let file: FileDiff | undefined;
      findHost = createPierreFindHost({
        getRoot: () => pierreShadowRoot(content),
        wrapper: root,
      });
      try {
        const fileDiff = historyFileDiff(path, original, modified, cacheKey);
        file = new FileDiff(
          {
            ...buildPierreDiffOptions({
              themeId,
              themeType,
              wordWrap,
              diffMode: mode,
              enableLineSelection: false,
              ...diffSettings(),
            }),
            ...readOnlyBlame(setCursorLine),
            onPostRender: (node: HTMLElement, instance: FileDiff, phase: PostRenderPhase) => {
              refreshFindAfterRender(findHost)(node, instance, phase);
              if (phase !== "unmount") scrollHost?.finishRender();
            },
          },
          getPierreWorkerPool()
        );
        scrollHost?.beginRender();
        file.render({ fileDiff, containerWrapper: content });
        scrollHost?.sync();
        fileRef = file;
      } catch (error) {
        setError(String(error));
      }

      return () => {
        findHost?.dispose();
        fileRef = undefined;
        file?.cleanUp();
      };
    }
  );

  createEffect(
    () =>
      [currentTheme().id, currentTheme().type, normalizeWordWrap(editorSettings().wordWrap), diffSettings()] as const,
    ([themeId, themeType, wordWrap, currentDiffSettings]) => {
      if (!fileRef) return;
      fileRef.setOptions({
        ...fileRef.options,
        ...buildPierreDiffOptions({
          themeId,
          themeType,
          wordWrap,
          diffMode: currentDiffSettings.layout,
          enableLineSelection: false,
          ...currentDiffSettings,
        }),
      });
      fileRef.rerender();
    }
  );

  return (
    <PierreScrollHost
      style={pierreHostStyle(editorSettings())}
      rootRef={(element) => {
        root = element;
      }}
      contentRef={(element) => {
        content = element;
      }}
      handleRef={(handle) => {
        scrollHost = handle;
      }}
    />
  );
};

export const PierreFileDiff = (props: PierreFileDiffProps) =>
  "original" in props ? <PierreHistoryFileDiff {...props} /> : <PierreScmFileDiff {...props} />;
