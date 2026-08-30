import { createEffect, createSignal, onSettled } from "solid-js";
import {
  FileDiff,
  parsePatchFiles,
  type DiffLineAnnotation,
  type FileContents,
  type FileDiffMetadata,
  type SelectedLineRange,
} from "@pierre/diffs";
import { Editor } from "@pierre/diffs/edit";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { DiffContent, DiffHunk, RepositoryStatus, ResourceGroupKind } from "../../lib/git-types";
import * as commands from "../../lib/tauri-commands";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { buildPierreDiffOptions } from "../../lib/pierre/options";
import { normalizeWordWrap } from "../../lib/pierre/normalize-editor-settings";
import { pierreEditorKeymap } from "../../lib/pierre/keymap";
import { getPierreWorkerPool } from "../../lib/pierre/worker";
import { flushPath, registerFlusher, trackPendingFlush } from "../../lib/pierre/flush-registry";
import { hunkActionAnchor, hunkIdentity, reidentifyHunk, type HunkIdentity } from "../../lib/pierre/hunk-annotations";
import { mapSelectionToStageLines, normalizeSelectionRange, type StageLinesCall } from "../../lib/pierre/line-map";
import { isDirty, sessionCacheKey, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";
import { selectionIsInPierreHost } from "./pierre-file";

const SAVE_MS = 1000;

export type HunkAnnotationMeta = { hunkIndex: number; identity: HunkIdentity };

export type PierreFileDiffProps = {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
};

export type ScmSessionHandle = {
  session: SaveSession;
  reload: (diff: DiffContent, incomingSha: string) => void;
};

let scmHandle: ScmSessionHandle | null = null;

export const registerScmSession = (handle: ScmSessionHandle): (() => void) => {
  scmHandle = handle;
  return () => {
    if (scmHandle === handle) scmHandle = null;
  };
};

export const getScmSession = (): ScmSessionHandle | null => scmHandle;

export const isScmDiffEditable = (groupKind: ResourceGroupKind, hasWorkingTreeSide: boolean): boolean =>
  groupKind !== "index" && groupKind !== "merge" && hasWorkingTreeSide;

export const enableScmLineSelection = (groupKind: ResourceGroupKind): boolean =>
  groupKind === "workingTree" || groupKind === "untracked" || groupKind === "index";

export const isNonPierreFileType = (fileType: string): boolean =>
  fileType === "image" || fileType === "binary" || fileType === "large";

export type EmptyPatchSides =
  | { oldFile: null; newFile: FileContents }
  | { oldFile: FileContents; newFile: null }
  | { oldFile: FileContents; newFile: FileContents };

export const emptyPatchSides = (
  path: string,
  cacheKey: string,
  original: string,
  modified: string
): EmptyPatchSides => {
  if (original === "") {
    return { oldFile: null, newFile: { name: path, contents: modified, cacheKey } };
  }
  if (modified === "") {
    return { oldFile: { name: path, contents: original, cacheKey }, newFile: null };
  }
  return {
    oldFile: { name: path, contents: original, cacheKey },
    newFile: { name: path, contents: modified, cacheKey },
  };
};

export const hunkAnnotations = (hunks: DiffHunk[]): DiffLineAnnotation<HunkAnnotationMeta>[] => {
  const annotations: DiffLineAnnotation<HunkAnnotationMeta>[] = [];
  for (const [hunkIndex, hunk] of hunks.entries()) {
    const anchor = hunkActionAnchor(hunk);
    if (!anchor) continue;
    annotations.push({
      side: anchor.side,
      lineNumber: anchor.lineNumber,
      metadata: { hunkIndex, identity: hunkIdentity(hunk) },
    });
  }
  return annotations;
};

export const runStageLineCalls = async (input: {
  path: string;
  staged: boolean;
  hunks: DiffHunk[];
  calls: StageLinesCall[];
  getFileHunks: (path: string, staged: boolean) => Promise<{ hunks: DiffHunk[] }>;
  stageLines: (
    path: string,
    hunkIndex: number,
    lineStart: number,
    lineEnd: number,
    staged: boolean
  ) => Promise<RepositoryStatus>;
  onStatus: (status: RepositoryStatus) => void;
  onWrote?: () => void;
}): Promise<RepositoryStatus | null> => {
  const pending = input.calls.map((call) => ({
    identity: hunkIdentity(input.hunks[call.hunkIndex]),
    lineStart: call.lineStart,
    lineEnd: call.lineEnd,
  }));
  let current = input.hunks;
  let last: RepositoryStatus | null = null;
  try {
    for (const [index, call] of pending.entries()) {
      const hunkIndex = reidentifyHunk(current, call.identity);
      if (hunkIndex === null) continue;
      last = await input.stageLines(input.path, hunkIndex, call.lineStart, call.lineEnd, input.staged);
      input.onStatus(last);
      if (index < pending.length - 1) {
        current = (await input.getFileHunks(input.path, input.staged)).hunks;
      }
    }
    return last;
  } finally {
    if (last) input.onWrote?.();
  }
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
  identity: HunkIdentity,
  groupKind: ResourceGroupKind,
  run: (identity: HunkIdentity, action: "stage" | "unstage" | "discard") => void
): HTMLElement | undefined => {
  const row = document.createElement("div");
  row.style.cssText = "display:inline-flex;align-items:center;gap:4px;";
  if (groupKind === "index") {
    row.append(hunkButton("Unstage", () => run(identity, "unstage")));
    return row;
  }
  row.append(
    hunkButton("Stage", () => run(identity, "stage")),
    hunkButton("Discard", () => run(identity, "discard"))
  );
  return row;
};

export const PierreFileDiff = (props: PierreFileDiffProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const diffMode = useStore(layoutStore, (s) => s.diffMode);
  const [ready, setReady] = createSignal(false);
  const [cacheGeneration, setCacheGeneration] = createSignal(0);
  const [viewGeneration, setViewGeneration] = createSignal(0);
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let session: SaveSession | null = null;
  let fileRef: FileDiff | undefined;

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
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
      ready() ? ([props.path, props.staged, props.groupKind, cacheGeneration(), viewGeneration()] as const) : null,
    (deps) => {
      if (!deps || !session) return;

      const [path, staged, groupKind] = deps;
      const mountedGeneration = session.cacheGeneration;
      const activeSession = session;
      const { setStatus, setError, setIsDiffDirty, setCursorLine, setDiff } = repositoryStore.getState();
      const themeId = currentTheme().id;
      const wordWrap = normalizeWordWrap(editorSettings().wordWrap);
      const mode = diffMode();

      let cancelled = false;
      let pendingTimer: ReturnType<typeof setTimeout> | null = null;
      let pendingText: string | null = null;
      let writeTail: Promise<void> = Promise.resolve();
      let unregisterFlush: (() => void) | undefined;
      let disposeEdit: (() => void) | undefined;
      let file: FileDiff | undefined;
      let editor: Editor<undefined> | undefined;
      let busy = false;

      const syncDirty = (): void => {
        setIsDiffDirty(isDirty({ pendingTimer: pendingTimer !== null, pendingSha: activeSession.pendingSha }));
      };

      const writeOnce = (text: string): Promise<void> => {
        writeTail = writeTail.then(async () => {
          activeSession.pendingSha = await sha256Utf8(text);
          syncDirty();
          try {
            await commands.writeFile(path, text);
            activeSession.diskSha = activeSession.pendingSha;
            activeSession.pendingSha = null;
            if (pendingText === text) pendingText = null;
            syncDirty();
          } catch (error) {
            setError(String(error));
            activeSession.pendingSha = null;
            syncDirty();
          }
        });
        return writeTail;
      };

      const scheduleSave = (text: string): void => {
        pendingText = text;
        if (pendingTimer) clearTimeout(pendingTimer);
        pendingTimer = setTimeout(() => {
          pendingTimer = null;
          const next = pendingText;
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
        if (pendingText !== null) {
          await writeOnce(pendingText);
          return;
        }
        await writeTail;
      };

      const afterGitWrite = (status: Awaited<ReturnType<typeof commands.stageHunk>>): void => {
        setStatus(status);
        setViewGeneration((value) => value + 1);
      };

      const runHunkAction = async (identity: HunkIdentity, action: "stage" | "unstage" | "discard"): Promise<void> => {
        if (busy) return;
        if (action === "discard") {
          const confirmed = await confirm(
            "Are you sure you want to discard this hunk?\n\nThis action is irreversible.",
            {
              title: "Discard Changes",
              kind: "warning",
              okLabel: "Discard",
              cancelLabel: "Cancel",
            }
          );
          if (!confirmed) return;
        }
        busy = true;
        try {
          await flushPath(path);
          const { hunks } = await commands.getFileHunks(path, staged);
          const hunkIndex = reidentifyHunk(hunks, identity);
          if (hunkIndex === null) return;
          const status =
            action === "discard"
              ? await commands.discardHunk(path, hunkIndex)
              : await commands.stageHunk(path, hunkIndex, action === "unstage");
          afterGitWrite(status);
        } catch (error) {
          setError(String(error));
        } finally {
          busy = false;
        }
      };

      const runLineSelection = async (range: SelectedLineRange): Promise<void> => {
        if (busy) return;
        busy = true;
        try {
          await flushPath(path);
          const { hunks } = await commands.getFileHunks(path, staged);
          const calls = mapSelectionToStageLines(hunks, normalizeSelectionRange(range));
          await runStageLineCalls({
            path,
            staged: groupKind === "index",
            hunks,
            calls,
            getFileHunks: commands.getFileHunks,
            stageLines: commands.stageLines,
            onStatus: setStatus,
            onWrote: () => setViewGeneration((value) => value + 1),
          });
        } catch (error) {
          setError(String(error));
        } finally {
          busy = false;
        }
      };

      const onSelectionChange = (): void => {
        if (!editor) return;
        const node = document.getSelection()?.anchorNode;
        if (!node) return;
        if (!selectionIsInPierreHost(root, node)) return;
        const start = editor.getState().selections?.[0]?.start;
        if (start) setCursorLine(start.line + 1);
      };

      void (async () => {
        const diff = await commands.getFileDiff(path, staged);
        if (cancelled) return;
        if (isNonPierreFileType(diff.fileType)) return;
        if (activeSession.diskSha === "") {
          activeSession.diskSha = await sha256Utf8(diff.modified);
        }
        if (cancelled) return;
        setDiff(diff);

        const patch = await commands.getFilePatch(path, staged);
        if (cancelled) return;

        const cacheKey = sessionCacheKey(activeSession);
        const sides = emptyPatchSides(path, cacheKey, diff.original, diff.modified);
        const editable = isScmDiffEditable(groupKind, sides.newFile !== null);
        let fileDiff: FileDiffMetadata | undefined;
        let annotations: DiffLineAnnotation<HunkAnnotationMeta>[] = [];

        if (patch.trim() !== "") {
          fileDiff = parsePatchFiles(patch)[0]?.files[0];
          annotations = hunkAnnotations((await commands.getFileHunks(path, staged)).hunks);
          if (cancelled) return;
        }

        const options = {
          ...buildPierreDiffOptions({
            themeId,
            wordWrap,
            diffMode: mode,
            enableLineSelection: enableScmLineSelection(groupKind),
          }),
          loadDiffFiles: fileDiff
            ? async () => ({
                oldFile: { name: path, contents: diff.original, cacheKey },
                newFile: { name: path, contents: diff.modified, cacheKey },
              })
            : undefined,
          renderAnnotation: (annotation: DiffLineAnnotation) => {
            const identity = annotations.find(
              (item) => item.side === annotation.side && item.lineNumber === annotation.lineNumber
            )?.metadata.identity;
            if (!identity) return;
            return renderHunkButtons(identity, groupKind, (next, action) => {
              void runHunkAction(next, action);
            });
          },
          onLineSelectionEnd: (range: SelectedLineRange | null) => {
            if (!range) return;
            void runLineSelection(range);
          },
        };

        file = new FileDiff(options, getPierreWorkerPool());
        if (fileDiff) {
          file.render({
            fileDiff,
            lineAnnotations: annotations.map((item) => ({ side: item.side, lineNumber: item.lineNumber })),
            containerWrapper: content,
          });
        } else if (sides.newFile === null) {
          file.render({ oldFile: sides.oldFile, newFile: null, containerWrapper: content });
        } else if (sides.oldFile === null) {
          file.render({ oldFile: null, newFile: sides.newFile, containerWrapper: content });
        } else {
          file.render({ oldFile: sides.oldFile, newFile: sides.newFile, containerWrapper: content });
        }
        fileRef = file;

        if (editable) {
          unregisterFlush = registerFlusher(path, flush);
          editor = new Editor<undefined>({
            persistState: false,
            keymap: pierreEditorKeymap,
            onChange(next) {
              scheduleSave(next.contents);
            },
          });
          disposeEdit = editor.edit(file);
          document.addEventListener("selectionchange", onSelectionChange);
        }
      })().catch((error: unknown) => {
        if (!cancelled) setError(String(error));
      });

      return () => {
        cancelled = true;
        document.removeEventListener("selectionchange", onSelectionChange);
        if (activeSession.cacheGeneration === mountedGeneration) {
          void trackPendingFlush(flush());
        } else if (pendingTimer) {
          clearTimeout(pendingTimer);
        }
        unregisterFlush?.();
        fileRef = undefined;
        disposeEdit?.();
        editor?.cleanUp();
        file?.cleanUp();
      };
    }
  );

  createEffect(
    () => [currentTheme().id, normalizeWordWrap(editorSettings().wordWrap), diffMode()] as const,
    ([themeId, wordWrap, mode]) => {
      if (!fileRef) return;
      fileRef.setOptions({
        ...fileRef.options,
        ...buildPierreDiffOptions({
          themeId,
          wordWrap,
          diffMode: mode,
          enableLineSelection: enableScmLineSelection(props.groupKind),
        }),
      });
    }
  );

  return (
    <div
      ref={(element) => {
        root = element;
      }}
      class="pierre-file-host"
      style={{ width: "100%", height: "100%", overflow: "auto" }}
    >
      <div
        ref={(element) => {
          content = element;
        }}
        class="pierre-file-content"
      />
    </div>
  );
};
