import { editor as MonacoEditor } from "monaco-editor";
import type { EditorSettings } from "../stores/settings-store";

export const buildDiffModelOptions = (editor: EditorSettings): MonacoEditor.ITextModelUpdateOptions => ({
  tabSize: editor.tabSize,
});

export const buildDiffOptions = (editor: EditorSettings, diffMode: "inline" | "sideBySide") =>
  ({
    renderSideBySide: diffMode === "sideBySide",
    minimap: { enabled: false },
    scrollBeyondLastLine: false,
    find: {
      addExtraSpaceOnTop: false,
      autoFindInSelection: "never" as const,
      seedSearchStringFromSelection: "always" as const,
    },
    fontSize: editor.fontSize,
    fontFamily: editor.fontFamily,
    lineHeight: editor.lineHeight,
    wordWrap: editor.wordWrap,
    renderWhitespace: editor.renderWhitespace,
    renderOverviewRuler: true,
    hideCursorInOverviewRuler: true,
    originalEditable: false,
    quickSuggestions: false,
    parameterHints: { enabled: false },
    suggestOnTriggerCharacters: false,
    codeLens: false,
    stickyScroll: { enabled: false },
    hover: { enabled: "off" },
    inlayHints: { enabled: "off" as const },
    glyphMargin: false,
    lineNumbersMinChars: 3,
    folding: false,
    matchBrackets: "never" as const,
    occurrencesHighlight: "off" as const,
    selectionHighlight: false,
    links: false,
    lightbulb: { enabled: MonacoEditor.ShowLightbulbIconMode.Off },
    bracketPairColorization: { enabled: false },
  }) satisfies MonacoEditor.IDiffEditorConstructionOptions;
