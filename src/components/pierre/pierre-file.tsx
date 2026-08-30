import { createEffect, createSignal, onSettled } from "solid-js";
import { DEFAULT_VIRTUAL_FILE_METRICS, VirtualizedFile, Virtualizer, type FileOptions } from "@pierre/diffs";
import { Editor } from "@pierre/diffs/edit";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { writeFile } from "../../lib/tauri-commands";
import { useStore } from "../../lib/use-store";
import { buildPierreDiffOptions } from "../../lib/pierre/options";
import { pierreThemeType } from "../../lib/pierre/theme";
import { normalizeWordWrap, pierreHostStyle } from "../../lib/pierre/normalize-editor-settings";
import { pierreEditorKeymap } from "../../lib/pierre/keymap";
import { getPierreWorkerPool } from "../../lib/pierre/worker";
import { registerFlusher, trackPendingFlush } from "../../lib/pierre/flush-registry";
import { isDirty, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";
import { commitPierreWrite } from "../../lib/pierre/buffered-write";
import { PierreScrollHost, type PierreScrollHostHandle } from "./pierre-scroll-host";

const SAVE_MS = 1000;

export const selectionIsInPierreHost = (root: HTMLElement, node: Node): boolean => {
  if (root.contains(node) || root.shadowRoot?.contains(node)) return true;
  for (const container of root.querySelectorAll("diffs-container")) {
    if (container.shadowRoot?.contains(node)) return true;
  }
  return false;
};

export type PierreFileProps = {
  path: string;
  contents: string;
  cacheKey: string;
  revealLine: number | null;
  session: SaveSession;
};

const fileOptionsFromSettings = (
  themeId: string,
  themeType: "light" | "dark",
  wordWrap: "off" | "on",
  onPostRender: NonNullable<FileOptions<undefined>["onPostRender"]>
): FileOptions<undefined> => {
  const diffOptions = buildPierreDiffOptions({
    themeId,
    themeType,
    wordWrap,
    diffMode: "inline",
    enableLineSelection: true,
  });
  return {
    theme: themeId,
    themeType: diffOptions.themeType,
    preferredHighlighter: diffOptions.preferredHighlighter,
    disableFileHeader: diffOptions.disableFileHeader,
    overflow: diffOptions.overflow,
    unsafeCSS: diffOptions.unsafeCSS,
    enableLineSelection: diffOptions.enableLineSelection,
    onPostRender,
  };
};

export const PierreFile = (props: PierreFileProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [ready, setReady] = createSignal(false);
  const [runtimeReady, setRuntimeReady] = createSignal(false);
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let editorRef: Editor<undefined> | undefined;
  let fileRef: VirtualizedFile | undefined;
  let activeSave: { path: string; schedule: (text: string) => void } | undefined;
  let scrollHost: PierreScrollHostHandle | undefined;

  const finishRender: NonNullable<FileOptions<undefined>["onPostRender"]> = (_node, _instance, phase) => {
    if (phase !== "unmount") scrollHost?.finishRender();
  };

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => ready(),
    (isReady) => {
      if (!isReady) return;

      const settings = settingsStore.getState().settings.editor;
      const theme = themeStore.getState().currentTheme;
      const virtualizer = new Virtualizer();
      virtualizer.setup(root, content);

      const file = new VirtualizedFile(
        fileOptionsFromSettings(
          theme.id,
          pierreThemeType(theme.kind),
          normalizeWordWrap(settings.wordWrap),
          finishRender
        ),
        virtualizer,
        {
          ...DEFAULT_VIRTUAL_FILE_METRICS,
          lineHeight: settings.lineHeight,
          paddingTop: 0,
          paddingBottom: 0,
        },
        getPierreWorkerPool()
      );
      const { setCursorLine } = repositoryStore.getState();
      const editor = new Editor<undefined>({
        persistState: true,
        keymap: pierreEditorKeymap,
        onChange(next) {
          if (activeSave?.path === next.name) activeSave.schedule(next.contents);
        },
      });
      const disposeEdit = editor.edit(file);
      fileRef = file;
      editorRef = editor;

      const onSelectionChange = (): void => {
        const node = document.getSelection()?.anchorNode;
        if (!node || !selectionIsInPierreHost(root, node)) return;
        const start = editor.getState().selections?.[0]?.start;
        if (start) setCursorLine(start.line + 1);
      };
      document.addEventListener("selectionchange", onSelectionChange);
      setRuntimeReady(true);

      return () => {
        setRuntimeReady(false);
        document.removeEventListener("selectionchange", onSelectionChange);
        activeSave = undefined;
        editorRef = undefined;
        fileRef = undefined;
        disposeEdit();
        file.cleanUp();
        virtualizer.cleanUp();
      };
    }
  );

  createEffect(
    () => (runtimeReady() ? ([props.path, props.contents, props.cacheKey, props.session] as const) : null),
    (deps) => {
      if (!deps || !fileRef) return;

      const [path, contents, cacheKey, session] = deps;
      const mountedGeneration = session.cacheGeneration;
      const { setIsFileDirty } = explorerStore.getState();
      const { setError } = repositoryStore.getState();
      let pendingTimer: ReturnType<typeof setTimeout> | null = null;
      const pending = { text: null as string | null };
      let writeTail: Promise<void> = Promise.resolve();

      const syncDirty = (): void => {
        setIsFileDirty(isDirty({ pendingTimer: pendingTimer !== null, pendingSha: session.pendingSha }));
      };

      const writeOnce = (text: string): Promise<void> => {
        const work = async (): Promise<void> => {
          try {
            await commitPierreWrite({
              writeFile: () => writeFile(path, text),
              pending,
              text,
              session,
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

      const schedule = (text: string): void => {
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

      const save = { path, schedule };
      activeSave = save;
      const unregister = registerFlusher(path, flush);
      scrollHost?.beginRender();
      fileRef.render({
        file: { name: path, contents, cacheKey },
        containerWrapper: content,
      });
      scrollHost?.sync();

      return () => {
        if (activeSave === save) activeSave = undefined;
        if (session.cacheGeneration === mountedGeneration) {
          void trackPendingFlush(path, flush()).catch(() => undefined);
        } else if (pendingTimer) {
          clearTimeout(pendingTimer);
        }
        unregister();
      };
    }
  );

  createEffect(
    () =>
      [currentTheme().id, pierreThemeType(currentTheme().kind), normalizeWordWrap(editorSettings().wordWrap)] as const,
    ([themeId, themeType, wordWrap]) => {
      fileRef?.setOptions(fileOptionsFromSettings(themeId, themeType, wordWrap, finishRender));
    }
  );

  createEffect(
    () => editorSettings().lineHeight,
    (lineHeight) => {
      fileRef?.setMetrics({
        ...DEFAULT_VIRTUAL_FILE_METRICS,
        lineHeight,
        paddingTop: 0,
        paddingBottom: 0,
      });
    }
  );

  createEffect(
    () => props.revealLine,
    (line) => {
      if (!line || !editorRef) return;
      editorRef.focus({ lineNumber: line });
      explorerStore.getState().setRevealLine(null);
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
