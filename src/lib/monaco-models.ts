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

export const disposeOwnedModel = (model: MonacoEditor.ITextModel | null | undefined, retain: boolean): void => {
  if (!model || retain || model.isAttachedToEditor()) {
    return;
  }
  model.dispose();
};

export const replaceEditorModel = (
  editor: Pick<MonacoEditor.IStandaloneCodeEditor, "getModel" | "setModel">,
  next: MonacoEditor.ITextModel,
  retainPrevious: boolean
): void => {
  const previous = editor.getModel();
  if (previous === next) {
    return;
  }
  editor.setModel(next);
  disposeOwnedModel(previous, retainPrevious);
};

export const disposeOwnedDiffModels = (
  models: { original?: MonacoEditor.ITextModel | null; modified?: MonacoEditor.ITextModel | null } | null | undefined,
  retain: { original: boolean; modified: boolean }
): void => {
  const original = models?.original;
  const modified = models?.modified;
  if (original && original === modified) {
    disposeOwnedModel(original, retain.original || retain.modified);
    return;
  }
  disposeOwnedModel(original, retain.original);
  disposeOwnedModel(modified, retain.modified);
};

export const replaceDiffEditorModels = (
  editor: Pick<MonacoEditor.IStandaloneDiffEditor, "getModel" | "setModel">,
  next: { original: MonacoEditor.ITextModel; modified: MonacoEditor.ITextModel },
  retainPrevious: { original: boolean; modified: boolean }
): void => {
  const previous = editor.getModel();
  if (previous?.original === next.original && previous?.modified === next.modified) {
    return;
  }
  editor.setModel(next);
  disposeOwnedDiffModels(previous, retainPrevious);
};
