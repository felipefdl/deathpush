import { createEffect, onSettled } from "solid-js";
import * as monaco from "monaco-editor";
import { applyDiffModelOptions, getOrCreateModel, setModelValueIfChanged } from "../../lib/monaco-models";

export type MonacoDiffEditorProps = {
  original: string;
  modified: string;
  language?: string;
  originalLanguage?: string;
  modifiedLanguage?: string;
  originalPath?: string;
  modifiedPath?: string;
  theme?: string;
  options?: monaco.editor.IStandaloneDiffEditorConstructionOptions & { tabSize?: number };
  keepCurrentOriginalModel?: boolean;
  keepCurrentModifiedModel?: boolean;
  onMount?: (editor: monaco.editor.IStandaloneDiffEditor, monacoApi: typeof monaco) => void;
};

export const MonacoDiffEditor = (props: MonacoDiffEditorProps) => {
  let container!: HTMLDivElement;
  let editor: monaco.editor.IStandaloneDiffEditor | undefined;

  const originalUri = (): monaco.Uri =>
    monaco.Uri.parse(props.originalPath ? `inmemory://model/${props.originalPath}` : "inmemory://model/original");
  const modifiedUri = (): monaco.Uri =>
    monaco.Uri.parse(props.modifiedPath ? `inmemory://model/${props.modifiedPath}` : "inmemory://model/modified");

  const syncModels = (): void => {
    if (!editor) return;
    const originalLanguage = props.originalLanguage ?? props.language;
    const modifiedLanguage = props.modifiedLanguage ?? props.language;
    const original = getOrCreateModel(props.original, originalLanguage, originalUri());
    const modified = getOrCreateModel(props.modified, modifiedLanguage, modifiedUri());
    const current = editor.getModel();
    if (current?.original !== original || current?.modified !== modified) {
      editor.setModel({ original, modified });
    } else {
      setModelValueIfChanged(original, props.original);
      setModelValueIfChanged(modified, props.modified);
    }
  };

  onSettled(() => {
    const originalLanguage = props.originalLanguage ?? props.language;
    const modifiedLanguage = props.modifiedLanguage ?? props.language;
    const original = getOrCreateModel(props.original, originalLanguage, originalUri());
    const modified = getOrCreateModel(props.modified, modifiedLanguage, modifiedUri());
    editor = monaco.editor.createDiffEditor(container, {
      ...props.options,
      theme: props.theme,
      automaticLayout: props.options?.automaticLayout ?? true,
    });
    editor.setModel({ original, modified });
    props.onMount?.(editor, monaco);
    if (props.options && "tabSize" in props.options && typeof props.options.tabSize === "number") {
      applyDiffModelOptions(editor, { tabSize: props.options.tabSize });
    }

    return () => {
      const current = editor;
      editor = undefined;
      const models = current?.getModel();
      current?.dispose();
      if (!props.keepCurrentOriginalModel) {
        models?.original.dispose();
      }
      if (!props.keepCurrentModifiedModel) {
        models?.modified.dispose();
      }
    };
  });

  createEffect(
    () =>
      [
        props.original,
        props.modified,
        props.language,
        props.originalLanguage,
        props.modifiedLanguage,
        props.originalPath,
        props.modifiedPath,
      ] as const,
    () => {
      syncModels();
    }
  );

  createEffect(
    () => props.theme,
    (theme) => {
      if (theme) monaco.editor.setTheme(theme);
    }
  );

  createEffect(
    () => props.options,
    (options) => {
      if (!options || !editor) return;
      editor.updateOptions(options);
      if ("tabSize" in options && typeof options.tabSize === "number") {
        applyDiffModelOptions(editor, { tabSize: options.tabSize });
      }
    }
  );

  return (
    <div
      ref={(element) => (container = element)}
      class="monaco-diff-editor-host"
      style={{ width: "100%", height: "100%" }}
    />
  );
};
