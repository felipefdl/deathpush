import { createEffect, createMemo, For, onSettled } from "solid-js";
import { editor as MonacoEditor } from "monaco-editor";
import type * as monaco from "monaco-editor";
import { explorerStore } from "../../stores/explorer-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useColorScheme } from "../../hooks/use-color-scheme";
import * as commands from "../../lib/tauri-commands";
import { writeFile } from "../../lib/tauri-commands";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { MonacoEditor as DeathPushMonacoEditor } from "../monaco/monaco-editor";

export const FileViewer = () => {
  const fileContent = useStore(explorerStore, (s) => s.fileContent);
  const selectedPath = useStore(explorerStore, (s) => s.selectedPath);
  const isFileDirty = useStore(explorerStore, (s) => s.isFileDirty);
  const revealLine = useStore(explorerStore, (s) => s.revealLine);
  const { setIsFileDirty } = explorerStore.getState();
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const colorScheme = useColorScheme();
  let disposeActions: (() => void) | undefined;
  let knownContent: string | null = null;
  let editorRef: monaco.editor.IStandaloneCodeEditor | undefined;

  createEffect(
    () => fileContent(),
    (content) => {
      knownContent = content?.content ?? null;
      setIsFileDirty(false);
    }
  );

  onSettled(() => {
    return () => {
      disposeActions?.();
    };
  });

  createEffect(
    () => [revealLine(), fileContent(), selectedPath()] as const,
    ([line, content, path]) => {
      if (!line || !editorRef) return;
      if (!content || content.path !== path) return;
      requestAnimationFrame(() => {
        const editor = editorRef;
        if (!editor) return;
        editor.revealLineInCenter(line);
        editor.setPosition({ lineNumber: line, column: 1 });
        editor.focus();
        explorerStore.getState().setRevealLine(null);
      });
    }
  );

  const handleMount = (editor: monaco.editor.IStandaloneCodeEditor, monacoApi: typeof monaco) => {
    editorRef = editor;
    disposeActions?.();

    const contentDisposable = editor.onDidChangeModelContent(() => {
      const current = editor.getValue();
      if (current === knownContent) return;
      setIsFileDirty(true);
    });

    const saveAction = editor.addAction({
      id: "deathpush.save",
      label: "Save File",
      keybindings: [monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyS],
      run: async () => {
        const state = explorerStore.getState();
        const path = state.selectedPath;
        const content = state.fileContent;
        if (!path || !content) return;
        const newContent = editor.getValue();
        try {
          await writeFile(path, newContent);
          knownContent = newContent;
          explorerStore.getState().setFileContent({ ...content, content: newContent });
          setIsFileDirty(false);
        } catch (e) {
          repositoryStore.getState().setError(String(e));
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

    const themeAction = editor.addAction({
      id: "deathpush.openThemePicker",
      label: "Open Theme Picker",
      keybindings: [chordKT],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
      },
    });

    const iconThemeAction = editor.addAction({
      id: "deathpush.openIconThemePicker",
      label: "Open Icon Theme Picker",
      keybindings: [chordKI],
      run: () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
      },
    });

    disposeActions = () => {
      contentDisposable.dispose();
      saveAction.dispose();
      themeAction.dispose();
      iconThemeAction.dispose();
    };
  };

  const editorOptions = createMemo(
    () =>
      ({
        minimap: { enabled: false },
        scrollBeyondLastLine: false,
        fontSize: editorSettings().fontSize,
        fontFamily: editorSettings().fontFamily,
        lineHeight: editorSettings().lineHeight,
        tabSize: editorSettings().tabSize,
        wordWrap: editorSettings().wordWrap,
        renderWhitespace: editorSettings().renderWhitespace,
        quickSuggestions: false,
        parameterHints: { enabled: false },
        suggestOnTriggerCharacters: false,
        codeLens: false,
        stickyScroll: { enabled: false },
        hover: { enabled: "off" },
        inlayHints: { enabled: "off" as const },
        glyphMargin: false,
        lineNumbersMinChars: 3,
        folding: true,
        matchBrackets: "never" as const,
        occurrencesHighlight: "off" as const,
        selectionHighlight: false,
        links: false,
        lightbulb: { enabled: MonacoEditor.ShowLightbulbIconMode.Off },
        bracketPairColorization: { enabled: false },
      }) satisfies MonacoEditor.IStandaloneEditorConstructionOptions
  );

  const handleOpenInEditor = async () => {
    const path = selectedPath();
    if (!path) return;
    try {
      await commands.openInEditor(path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleRevealInFinder = async () => {
    const path = selectedPath();
    if (!path) return;
    try {
      await commands.revealInFileManager(path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const breadcrumbs = createMemo(() => selectedPath()?.split("/") ?? []);
  const fileName = createMemo(() => {
    const path = selectedPath();
    return path ? (path.split("/").pop() ?? path) : "";
  });

  const headerActions = (includeReveal: boolean) => (
    <div class="diff-header-actions">
      {includeReveal && (
        <button class="scm-toolbar-button" onClick={handleRevealInFinder} title="Reveal in Finder">
          <span class="codicon codicon-folder-opened" />
        </button>
      )}
      <button class="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
        <span class="codicon codicon-go-to-file" />
      </button>
    </div>
  );

  const breadcrumbTrail = (showDirty: boolean) => (
    <span class="file-viewer-breadcrumbs" title={selectedPath() ?? ""}>
      <For each={breadcrumbs()} keyed={false}>
        {(part, index) => (
          <span>
            {index > 0 && <span class="file-viewer-separator"> / </span>}
            {part()}
          </span>
        )}
      </For>
      {showDirty && isFileDirty() && <span class="dirty-indicator"> *</span>}
    </span>
  );

  return (
    <>
      {!fileContent() || !selectedPath() ? (
        <div class="diff-empty-state">
          <img
            class="diff-empty-watermark"
            src={colorScheme() === "dark" ? "/deathpush-white.png" : "/deathpush-black.png"}
            alt=""
          />
          <p style={{ opacity: 0.4, "margin-top": "12px" }}>Select a file to view its contents</p>
        </div>
      ) : fileContent()!.fileType === "large" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(false)}
          </div>
          <div class="file-viewer-message">
            <span class="codicon codicon-warning" style={{ "font-size": "32px", opacity: 0.4 }} />
            <p>File is too large to display (over 5 MB)</p>
            <button class="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
              Open in External Editor
            </button>
          </div>
        </div>
      ) : fileContent()!.fileType === "binary" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(false)}
          </div>
          <div class="file-viewer-message">
            <span class="codicon codicon-file-binary" style={{ "font-size": "32px", opacity: 0.4 }} />
            <p>Binary file cannot be displayed</p>
            <button class="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
              Open in External Editor
            </button>
          </div>
        </div>
      ) : fileContent()!.fileType === "image" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(true)}
          </div>
          <div class="file-viewer-image">
            <img src={fileContent()!.content} alt={fileName()} />
          </div>
        </div>
      ) : (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(true)}
            {headerActions(true)}
          </div>
          <div class="diff-editor-container">
            <DeathPushMonacoEditor
              value={fileContent()!.content}
              language={fileContent()!.language ?? undefined}
              path={selectedPath() ?? undefined}
              theme={currentTheme().id}
              onMount={handleMount}
              options={editorOptions()}
            />
          </div>
        </div>
      )}
    </>
  );
};
