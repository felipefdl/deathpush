import type { EditorSettings } from "../stores/settings-store";

export const buildDiffOptions = (
  editor: EditorSettings,
  diffMode: "inline" | "sideBySide",
) => ({
  renderSideBySide: diffMode === "sideBySide",
  minimap: { enabled: false },
  scrollBeyondLastLine: false,
  find: { addExtraSpaceOnTop: false, autoFindInSelection: "never", seedSearchStringFromSelection: "always" },
  fontSize: editor.fontSize,
  fontFamily: editor.fontFamily,
  lineHeight: editor.lineHeight,
  // @ts-expect-error tabSize works at runtime but is missing from IDiffEditorConstructionOptions
  tabSize: editor.tabSize,
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
});
