import { createEffect, createSignal, onSettled } from "solid-js";
import { UnresolvedFile, type FileContents, type UnresolvedFileOptions } from "@pierre/diffs";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { buildPierreDiffOptions } from "../../lib/pierre/options";
import { normalizeWordWrap, pierreHostStyle } from "../../lib/pierre/normalize-editor-settings";
import { pierreThemeType } from "../../lib/pierre/theme";
import { getPierreWorkerPool } from "../../lib/pierre/worker";
import { sessionCacheKey, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";
import { stageFiles, writeFile } from "../../lib/tauri-commands";

export type PierreUnresolvedProps = {
  path: string;
  contents: string;
};

export const shouldMountMergePane = (
  selectedFile: { path: string; groupKind: string } | null,
  selectedLoadId: number,
  diff: { path: string } | null,
  diffLoadId: number | null
): boolean =>
  selectedFile?.groupKind === "merge" &&
  diff !== null &&
  diff.path === selectedFile.path &&
  diffLoadId === selectedLoadId;

const mergeResolveTails = new Map<string, Promise<void>>();

export const enqueueMergeResolve = (path: string, work: () => Promise<void>): Promise<void> => {
  const previous = mergeResolveTails.get(path) ?? Promise.resolve();
  const next = previous.then(work, work);
  mergeResolveTails.set(
    path,
    next.then(
      () => undefined,
      () => undefined
    )
  );
  return next;
};

const unresolvedOptions = (
  themeId: string,
  themeType: "light" | "dark",
  wordWrap: "off" | "on"
): UnresolvedFileOptions<undefined> => {
  const { diffStyle: _diffStyle, ...options } = buildPierreDiffOptions({
    themeId,
    themeType,
    wordWrap,
    diffMode: "inline",
    enableLineSelection: false,
  });
  return options;
};

export const PierreUnresolved = (props: PierreUnresolvedProps) => {
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [ready, setReady] = createSignal(false);
  let content!: HTMLDivElement;
  let fileRef: UnresolvedFile | undefined;
  let session: SaveSession | null = null;

  onSettled(() => {
    setReady(true);
    return () => setReady(false);
  });

  createEffect(
    () => (ready() ? ([props.path, props.contents] as const) : null),
    (deps) => {
      if (!deps) return;

      const [path, contents] = deps;
      const { setStatus, setError } = repositoryStore.getState();
      const theme = currentTheme();
      const themeId = theme.id;
      const themeType = pierreThemeType(theme.kind);
      const wordWrap = normalizeWordWrap(editorSettings().wordWrap);
      session = { path, diskSha: "", pendingSha: null, cacheGeneration: 0 };
      void sha256Utf8(contents).then((sha) => {
        if (session?.path === path) session.diskSha = sha;
      });

      const options: UnresolvedFileOptions<undefined> = {
        ...unresolvedOptions(themeId, themeType, wordWrap),
        mergeConflictActionsType: "default",
        onMergeConflictResolve: (file: FileContents) => {
          void enqueueMergeResolve(path, async () => {
            try {
              await writeFile(path, file.contents);
              if (session) session.diskSha = await sha256Utf8(file.contents);
              const status = await stageFiles([path]);
              setStatus(status);
            } catch (error) {
              setError(String(error));
            }
          });
        },
      };

      const file = new UnresolvedFile(options, getPierreWorkerPool());
      file.render({
        file: { name: path, contents, cacheKey: session ? sessionCacheKey(session) : path },
        containerWrapper: content,
      });
      fileRef = file;

      return () => {
        fileRef = undefined;
        file.cleanUp();
      };
    }
  );

  createEffect(
    () =>
      [currentTheme().id, pierreThemeType(currentTheme().kind), normalizeWordWrap(editorSettings().wordWrap)] as const,
    ([themeId, themeType, wordWrap]) => {
      if (!fileRef) return;
      fileRef.setOptions({
        ...fileRef.options,
        ...unresolvedOptions(themeId, themeType, wordWrap),
      });
    }
  );

  return (
    <div class="pierre-file-host" style={pierreHostStyle(editorSettings())}>
      <div
        ref={(element) => {
          content = element;
        }}
        class="pierre-file-content"
      />
    </div>
  );
};
