import { createEffect, createMemo, onSettled } from "solid-js";
import type * as monaco from "monaco-editor";
import { writeFile } from "../../lib/tauri-commands";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { buildDiffOptions } from "../../lib/diff-options";
import { MonacoDiffEditor } from "../monaco/monaco-diff-editor";
import { DiffHeader } from "./diff-header";
import { EmptyState } from "./empty-state";
import { ImageDiff } from "./image-diff";

export const DiffViewer = () => {
  const diff = useStore(repositoryStore, (s) => s.diff);
  const selectedFile = useStore(repositoryStore, (s) => s.selectedFile);
  const isDiffDirty = useStore(repositoryStore, (s) => s.isDiffDirty);
  const { setDiff, setError, setIsDiffDirty, setCursorLine } = repositoryStore.getState();
  const diffMode = useStore(layoutStore, (s) => s.diffMode);
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  let disposeActions: (() => void) | undefined;
  let knownModified: string | null = null;

  const isEditable = createMemo(() => {
    const file = selectedFile();
    return !!file && !file.staged;
  });

  createEffect(
    () => diff(),
    (next) => {
      knownModified = next?.modified ?? null;
      setIsDiffDirty(false);
    }
  );

  const handleMount = (editor: monaco.editor.IStandaloneDiffEditor, monacoApi: typeof monaco) => {
    disposeActions?.();

    const modified = editor.getModifiedEditor();

    const contentDisposable = modified.onDidChangeModelContent(() => {
      const current = modified.getValue();
      if (current === knownModified) return;
      setIsDiffDirty(true);
    });

    const cursorDisposable = modified.onDidChangeCursorPosition((e) => {
      setCursorLine(e.position.lineNumber);
    });

    const saveAction = modified.addAction({
      id: "deathpush.save",
      label: "Save File",
      keybindings: [monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyS],
      run: async () => {
        const state = repositoryStore.getState();
        const file = state.selectedFile;
        if (!file) return;
        const content = modified.getValue();
        try {
          await writeFile(file.path, content);
          const currentDiff = state.diff;
          knownModified = content;
          if (currentDiff) {
            setDiff({ ...currentDiff, modified: content });
          }
          setIsDiffDirty(false);
        } catch (e) {
          setError(String(e));
        }
      },
    });

    const chordKT = monacoApi.KeyMod.chord(
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyK,
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyT
    );
    const chordKI = monacoApi.KeyMod.chord(
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyK,
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyI
    );

    const themeAction = modified.addAction({
      id: "deathpush.openThemePicker",
      label: "Open Theme Picker",
      keybindings: [chordKT],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
      },
    });

    const iconThemeAction = modified.addAction({
      id: "deathpush.openIconThemePicker",
      label: "Open Icon Theme Picker",
      keybindings: [chordKI],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
      },
    });

    const original = editor.getOriginalEditor();

    const themeActionOrig = original.addAction({
      id: "deathpush.openThemePicker",
      label: "Open Theme Picker",
      keybindings: [chordKT],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
      },
    });

    const iconThemeActionOrig = original.addAction({
      id: "deathpush.openIconThemePicker",
      label: "Open Icon Theme Picker",
      keybindings: [chordKI],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
      },
    });

    disposeActions = () => {
      contentDisposable.dispose();
      cursorDisposable.dispose();
      saveAction.dispose();
      themeAction.dispose();
      iconThemeAction.dispose();
      themeActionOrig.dispose();
      iconThemeActionOrig.dispose();
    };
  };

  onSettled(() => {
    return () => {
      disposeActions?.();
    };
  });

  const editorOptions = createMemo(() => ({
    ...buildDiffOptions(editorSettings(), diffMode()),
    readOnly: !isEditable(),
    domReadOnly: !isEditable(),
    tabSize: editorSettings().tabSize,
  }));

  return (
    <>
      {!diff() || !selectedFile() ? (
        <EmptyState />
      ) : diff()!.fileType === "image" ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <ImageDiff original={diff()!.original} modified={diff()!.modified} />
        </div>
      ) : (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <div class="diff-editor-container">
            <MonacoDiffEditor
              original={diff()!.original}
              modified={diff()!.modified}
              originalPath={`original/${diff()!.path}`}
              modifiedPath={`modified/${diff()!.path}`}
              language={diff()!.originalLanguage ?? undefined}
              theme={currentTheme().id}
              keepCurrentOriginalModel
              keepCurrentModifiedModel
              onMount={handleMount}
              options={editorOptions()}
            />
          </div>
        </div>
      )}
    </>
  );
};
