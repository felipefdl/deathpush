import { DiffEditor, type DiffOnMount } from "@monaco-editor/react";
import { useCallback, useEffect, useRef } from "react";
import { writeFile } from "../../lib/tauri-commands";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { DiffHeader } from "./diff-header";
import { EmptyState } from "./empty-state";
import { ImageDiff } from "./image-diff";

export const DiffViewer = () => {
  const { diff, selectedFile, isDiffDirty, setDiff, setError, setIsDiffDirty, setCursorLine } =
    useRepositoryStore();
  const { diffMode } = useLayoutStore();
  const { settings } = useSettingsStore();
  const { currentTheme } = useThemeStore();
  const editorRef = useRef<Parameters<DiffOnMount>[0] | null>(null);
  const disposeRef = useRef<(() => void) | null>(null);
  const knownModifiedRef = useRef<string | null>(null);

  const isEditable = !!selectedFile && !selectedFile.staged;

  useEffect(() => {
    knownModifiedRef.current = diff?.modified ?? null;
    setIsDiffDirty(false);
  }, [diff, setIsDiffDirty]);

  const handleMount: DiffOnMount = useCallback(
    (editor, monaco) => {
      editorRef.current = editor;
      if (disposeRef.current) disposeRef.current();

      const modified = editor.getModifiedEditor();

      const contentDisposable = modified.onDidChangeModelContent(() => {
        const current = modified.getValue();
        if (current === knownModifiedRef.current) return;
        setIsDiffDirty(true);
      });

      const cursorDisposable = modified.onDidChangeCursorPosition((e) => {
        setCursorLine(e.position.lineNumber);
      });

      const saveAction = modified.addAction({
        id: "deathpush.save",
        label: "Save File",
        keybindings: [2048 | 49], // KeyMod.CtrlCmd | KeyCode.KeyS
        run: async () => {
          const state = useRepositoryStore.getState();
          const file = state.selectedFile;
          if (!file) return;
          const content = modified.getValue();
          try {
            await writeFile(file.path, content);
            const currentDiff = state.diff;
            knownModifiedRef.current = content;
            if (currentDiff) {
              setDiff({ ...currentDiff, modified: content });
            }
            setIsDiffDirty(false);
          } catch (e) {
            setError(String(e));
          }
        },
      });

      const chordKT = monaco.KeyMod.chord(
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyT,
      );
      const chordKI = monaco.KeyMod.chord(
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
        monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyI,
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

      disposeRef.current = () => {
        contentDisposable.dispose();
        cursorDisposable.dispose();
        saveAction.dispose();
        themeAction.dispose();
        iconThemeAction.dispose();
        themeActionOrig.dispose();
        iconThemeActionOrig.dispose();
      };

      // Explicitly set model content on mount to fix race condition where
      // @monaco-editor/react creates empty models before populating them
      const currentDiff = useRepositoryStore.getState().diff;
      if (currentDiff) {
        const origModel = editor.getOriginalEditor().getModel();
        const modModel = editor.getModifiedEditor().getModel();
        if (origModel) origModel.setValue(currentDiff.original);
        if (modModel) {
          knownModifiedRef.current = currentDiff.modified;
          modModel.setValue(currentDiff.modified);
        }
      }
    },
    [setDiff, setError, setIsDiffDirty, setCursorLine],
  );

  // Force-update original model when diff changes (handles same-file refresh from watcher)
  useEffect(() => {
    if (!editorRef.current || !diff) return;
    const origModel = editorRef.current.getOriginalEditor().getModel();
    if (origModel) origModel.setValue(diff.original);
  }, [diff]);

  useEffect(() => {
    return () => {
      if (disposeRef.current) disposeRef.current();
      // Dispose models to prevent stale content when the same file is reopened
      if (editorRef.current) {
        editorRef.current.getOriginalEditor().getModel()?.dispose();
        editorRef.current.getModifiedEditor().getModel()?.dispose();
        editorRef.current = null;
      }
    };
  }, []);

  if (!diff || !selectedFile) {
    return <EmptyState />;
  }

  if (diff.fileType === "image") {
    return (
      <div className="diff-viewer">
        <DiffHeader isDirty={isDiffDirty} />
        <ImageDiff original={diff.original} modified={diff.modified} />
      </div>
    );
  }

  return (
    <div className="diff-viewer">
      <DiffHeader isDirty={isDiffDirty} />
      <div className="diff-editor-container">
        <DiffEditor
          key={`${diff.path}:${selectedFile.staged}`}
          original={diff.original}
          modified={diff.modified}
          originalModelPath={`original/${diff.path}`}
          modifiedModelPath={`modified/${diff.path}`}
          language={diff.originalLanguage ?? undefined}
          theme={currentTheme.id}
          onMount={handleMount}
          options={{
            readOnly: !isEditable,
            domReadOnly: !isEditable,
            renderSideBySide: diffMode === "sideBySide",
            minimap: { enabled: settings.editor.minimap },
            scrollBeyondLastLine: false,
            fontSize: settings.editor.fontSize,
            fontFamily: settings.editor.fontFamily,
            lineHeight: settings.editor.lineHeight,
            // @ts-expect-error tabSize works at runtime but is missing from IDiffEditorConstructionOptions
            tabSize: settings.editor.tabSize,
            wordWrap: settings.editor.wordWrap,
            renderWhitespace: settings.editor.renderWhitespace,
            renderOverviewRuler: false,
            originalEditable: false,
            quickSuggestions: false,
            parameterHints: { enabled: false },
            suggestOnTriggerCharacters: false,
            codeLens: false,
            stickyScroll: { enabled: false },
            hover: { enabled: false },
            inlayHints: { enabled: "off" },
            glyphMargin: false,
            lineNumbersMinChars: 3,
            folding: false,
            matchBrackets: "never",
            occurrencesHighlight: "off",
            selectionHighlight: false,
            links: false,
            // eslint-disable-next-line @typescript-eslint/no-explicit-any
            lightbulb: { enabled: "off" as any },
            bracketPairColorization: { enabled: false },
          }}
        />
      </div>
    </div>
  );
};
