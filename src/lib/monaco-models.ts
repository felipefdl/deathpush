import type { editor as MonacoEditor } from "monaco-editor";
import * as monaco from "monaco-editor";

export const getOrCreateModel = (
  value: string,
  language: string | undefined,
  uri: monaco.Uri
): MonacoEditor.ITextModel => {
  const existing = monaco.editor.getModel(uri);
  if (existing) {
    if (existing.getValue() !== value) {
      existing.setValue(value);
    }
    if (language && existing.getLanguageId() !== language) {
      monaco.editor.setModelLanguage(existing, language);
    }
    return existing;
  }
  return monaco.editor.createModel(value, language, uri);
};

export const setModelValueIfChanged = (model: MonacoEditor.ITextModel, value: string): void => {
  if (model.getValue() !== value) {
    model.setValue(value);
  }
};

export const applyDiffModelOptions = (
  editor: MonacoEditor.IStandaloneDiffEditor,
  options: MonacoEditor.ITextModelUpdateOptions
): void => {
  editor.getOriginalEditor().getModel()?.updateOptions(options);
  editor.getModifiedEditor().getModel()?.updateOptions(options);
};
