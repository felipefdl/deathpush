import { Editor, type OnMount } from "@monaco-editor/react";
import { useCallback, useEffect, useMemo, useRef } from "react";
import { useExplorerStore } from "../../stores/explorer-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { useColorScheme } from "../../hooks/use-color-scheme";
import * as commands from "../../lib/tauri-commands";
import { writeFile } from "../../lib/tauri-commands";
import { useRepositoryStore } from "../../stores/repository-store";

export const FileViewer = () => {
  const fileContent = useExplorerStore((s) => s.fileContent);
  const selectedPath = useExplorerStore((s) => s.selectedPath);
  const isFileDirty = useExplorerStore((s) => s.isFileDirty);
  const setIsFileDirty = useExplorerStore((s) => s.setIsFileDirty);
  const { settings } = useSettingsStore();
  const { currentTheme } = useThemeStore();
  const setError = useRepositoryStore((s) => s.setError);
  const colorScheme = useColorScheme();
  const isDark = colorScheme === "dark";
  const disposeRef = useRef<(() => void) | null>(null);
  const knownContentRef = useRef<string | null>(null);

  useEffect(() => {
    knownContentRef.current = fileContent?.content ?? null;
    setIsFileDirty(false);
  }, [fileContent, setIsFileDirty]);

  useEffect(() => {
    return () => {
      if (disposeRef.current) disposeRef.current();
    };
  }, []);

  const handleMount: OnMount = useCallback((editor, monaco) => {
    if (disposeRef.current) disposeRef.current();

    const contentDisposable = editor.onDidChangeModelContent(() => {
      const current = editor.getValue();
      if (current === knownContentRef.current) return;
      setIsFileDirty(true);
    });

    const saveAction = editor.addAction({
      id: "deathpush.save",
      label: "Save File",
      keybindings: [monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyS],
      run: async () => {
        const state = useExplorerStore.getState();
        const path = state.selectedPath;
        const content = state.fileContent;
        if (!path || !content) return;
        const newContent = editor.getValue();
        try {
          await writeFile(path, newContent);
          knownContentRef.current = newContent;
          useExplorerStore.getState().setFileContent({ ...content, content: newContent });
          setIsFileDirty(false);
        } catch (e) {
          useRepositoryStore.getState().setError(String(e));
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

    disposeRef.current = () => {
      contentDisposable.dispose();
      saveAction.dispose();
      themeAction.dispose();
      iconThemeAction.dispose();
    };
  }, [setIsFileDirty]);

  const editorOptions = useMemo(
    () => ({
      minimap: { enabled: false },
      scrollBeyondLastLine: false,
      fontSize: settings.editor.fontSize,
      fontFamily: settings.editor.fontFamily,
      lineHeight: settings.editor.lineHeight,
      tabSize: settings.editor.tabSize,
      wordWrap: settings.editor.wordWrap,
      renderWhitespace: settings.editor.renderWhitespace,
      quickSuggestions: false,
      parameterHints: { enabled: false },
      suggestOnTriggerCharacters: false,
      codeLens: false,
      stickyScroll: { enabled: false },
      hover: { enabled: false },
      inlayHints: { enabled: "off" as const },
      glyphMargin: false,
      lineNumbersMinChars: 3,
      folding: true,
      matchBrackets: "never" as const,
      occurrencesHighlight: "off" as const,
      selectionHighlight: false,
      links: false,
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      lightbulb: { enabled: "off" as any },
      bracketPairColorization: { enabled: false },
    }),
    [settings.editor],
  );

  const handleOpenInEditor = useCallback(async () => {
    if (!selectedPath) return;
    try {
      await commands.openInEditor(selectedPath);
    } catch (err) {
      setError(String(err));
    }
  }, [selectedPath, setError]);

  const handleRevealInFinder = useCallback(async () => {
    if (!selectedPath) return;
    try {
      await commands.revealInFileManager(selectedPath);
    } catch (err) {
      setError(String(err));
    }
  }, [selectedPath, setError]);

  if (!fileContent || !selectedPath) {
    return (
      <div className="diff-empty-state">
        <img
          className="diff-empty-watermark"
          src={isDark ? "/deathpush-white.png" : "/deathpush-black.png"}
          alt=""
        />
        <p style={{ opacity: 0.4, marginTop: 12 }}>Select a file to view its contents</p>
      </div>
    );
  }

  const fileName = selectedPath.split("/").pop() ?? selectedPath;
  const breadcrumbs = selectedPath.split("/");

  if (fileContent.fileType === "large") {
    return (
      <div className="diff-viewer">
        <div className="file-viewer-header">
          <span className="file-viewer-breadcrumbs" title={selectedPath}>
            {breadcrumbs.map((part, i) => (
              <span key={i}>
                {i > 0 && <span className="file-viewer-separator"> / </span>}
                {part}
              </span>
            ))}
          </span>
          <div className="diff-header-actions">
            <button className="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
              <span className="codicon codicon-go-to-file" />
            </button>
          </div>
        </div>
        <div className="file-viewer-message">
          <span className="codicon codicon-warning" style={{ fontSize: 32, opacity: 0.4 }} />
          <p>File is too large to display (over 5 MB)</p>
          <button className="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
            Open in External Editor
          </button>
        </div>
      </div>
    );
  }

  if (fileContent.fileType === "binary") {
    return (
      <div className="diff-viewer">
        <div className="file-viewer-header">
          <span className="file-viewer-breadcrumbs" title={selectedPath}>
            {breadcrumbs.map((part, i) => (
              <span key={i}>
                {i > 0 && <span className="file-viewer-separator"> / </span>}
                {part}
              </span>
            ))}
          </span>
          <div className="diff-header-actions">
            <button className="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
              <span className="codicon codicon-go-to-file" />
            </button>
          </div>
        </div>
        <div className="file-viewer-message">
          <span className="codicon codicon-file-binary" style={{ fontSize: 32, opacity: 0.4 }} />
          <p>Binary file cannot be displayed</p>
          <button className="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
            Open in External Editor
          </button>
        </div>
      </div>
    );
  }

  if (fileContent.fileType === "image") {
    return (
      <div className="diff-viewer">
        <div className="file-viewer-header">
          <span className="file-viewer-breadcrumbs" title={selectedPath}>
            {breadcrumbs.map((part, i) => (
              <span key={i}>
                {i > 0 && <span className="file-viewer-separator"> / </span>}
                {part}
              </span>
            ))}
          </span>
          <div className="diff-header-actions">
            <button className="scm-toolbar-button" onClick={handleRevealInFinder} title="Reveal in Finder">
              <span className="codicon codicon-folder-opened" />
            </button>
            <button className="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
              <span className="codicon codicon-go-to-file" />
            </button>
          </div>
        </div>
        <div className="file-viewer-image">
          <img src={fileContent.content} alt={fileName} />
        </div>
      </div>
    );
  }

  return (
    <div className="diff-viewer">
      <div className="file-viewer-header">
        <span className="file-viewer-breadcrumbs" title={selectedPath}>
          {breadcrumbs.map((part, i) => (
            <span key={i}>
              {i > 0 && <span className="file-viewer-separator"> / </span>}
              {part}
            </span>
          ))}
          {isFileDirty && <span className="diff-header-dirty"> *</span>}
        </span>
        <div className="diff-header-actions">
          <button className="scm-toolbar-button" onClick={handleRevealInFinder} title="Reveal in Finder">
            <span className="codicon codicon-folder-opened" />
          </button>
          <button className="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
            <span className="codicon codicon-go-to-file" />
          </button>
        </div>
      </div>
      <div className="diff-editor-container">
        <Editor
          key={selectedPath}
          value={fileContent.content}
          language={fileContent.language ?? undefined}
          theme={currentTheme.id}
          onMount={handleMount}
          options={editorOptions}
        />
      </div>
    </div>
  );
};
