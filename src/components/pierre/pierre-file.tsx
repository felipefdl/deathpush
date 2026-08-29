import { createEffect, createSignal, onSettled } from "solid-js";
import { VirtualizedFile, Virtualizer, type FileOptions } from "@pierre/diffs";
import { Editor } from "@pierre/diffs/edit";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { writeFile } from "../../lib/tauri-commands";
import { useStore } from "../../lib/use-store";
import { buildPierreDiffOptions } from "../../lib/pierre/options";
import { normalizeWordWrap } from "../../lib/pierre/normalize-editor-settings";
import { pierreEditorKeymap } from "../../lib/pierre/keymap";
import { getPierreWorkerPool } from "../../lib/pierre/worker";
import { registerFlusher } from "../../lib/pierre/flush-registry";
import { isDirty, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";

const SAVE_MS = 1000;

export type PierreFileProps = {
  path: string;
  contents: string;
  cacheKey: string;
  revealLine: number | null;
  session: SaveSession;
};

const fileOptionsFromSettings = (themeId: string, wordWrap: "off" | "on"): FileOptions<undefined> => {
  const diffOptions = buildPierreDiffOptions({
    themeId,
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
    enableLineSelection: diffOptions.enableLineSelection,
  };
};

export const PierreFile = (props: PierreFileProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [ready, setReady] = createSignal(false);
  let root!: HTMLDivElement;
  let content!: HTMLDivElement;
  let editorRef: Editor<undefined> | undefined;
  let fileRef: VirtualizedFile | undefined;

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => (ready() ? ([props.path, props.cacheKey] as const) : null),
    (deps) => {
      if (!deps) return;

      const [path, cacheKey] = deps;
      const contents = props.contents;
      const session = props.session;
      const mountedGeneration = session.cacheGeneration;
      const { setIsFileDirty } = explorerStore.getState();
      const { setError, setCursorLine } = repositoryStore.getState();
      const themeId = currentTheme().id;
      const wordWrap = normalizeWordWrap(editorSettings().wordWrap);

      const virtualizer = new Virtualizer();
      virtualizer.setup(root, content);

      const file = new VirtualizedFile(
        fileOptionsFromSettings(themeId, wordWrap),
        virtualizer,
        undefined,
        getPierreWorkerPool()
      );
      file.render({
        file: { name: path, contents, cacheKey },
        containerWrapper: content,
      });
      fileRef = file;

      let pendingTimer: ReturnType<typeof setTimeout> | null = null;
      let pendingText: string | null = null;
      let writeTail: Promise<void> = Promise.resolve();

      const syncDirty = (): void => {
        setIsFileDirty(isDirty({ pendingTimer: pendingTimer !== null, pendingSha: session.pendingSha }));
      };

      const writeOnce = (text: string): Promise<void> => {
        writeTail = writeTail.then(async () => {
          session.pendingSha = await sha256Utf8(text);
          syncDirty();
          try {
            await writeFile(path, text);
            session.diskSha = session.pendingSha;
            session.pendingSha = null;
            if (pendingText === text) pendingText = null;
            syncDirty();
          } catch (error) {
            setError(String(error));
            session.pendingSha = null;
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

      const unregister = registerFlusher(path, flush);

      const editor = new Editor<undefined>({
        persistState: false,
        keymap: pierreEditorKeymap,
        onChange(next) {
          scheduleSave(next.contents);
        },
        onAttach() {
          const line = props.revealLine;
          if (line) {
            editor.focus({ lineNumber: line });
            explorerStore.getState().setRevealLine(null);
          }
        },
      });
      const disposeEdit = editor.edit(file);
      editorRef = editor;

      const onSelectionChange = (): void => {
        const node = document.getSelection()?.anchorNode;
        if (!node) return;
        if (!root.shadowRoot?.contains(node) && !root.contains(node)) return;
        const start = editor.getState().selections?.[0]?.start;
        if (start) setCursorLine(start.line + 1);
      };
      document.addEventListener("selectionchange", onSelectionChange);

      return () => {
        document.removeEventListener("selectionchange", onSelectionChange);
        if (session.cacheGeneration === mountedGeneration) {
          void flush();
        } else if (pendingTimer) {
          clearTimeout(pendingTimer);
        }
        unregister();
        editorRef = undefined;
        fileRef = undefined;
        disposeEdit();
        editor.cleanUp();
        file.cleanUp();
        virtualizer.cleanUp();
      };
    }
  );

  createEffect(
    () => [currentTheme().id, normalizeWordWrap(editorSettings().wordWrap)] as const,
    ([themeId, wordWrap]) => {
      fileRef?.setOptions(fileOptionsFromSettings(themeId, wordWrap));
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
