import { createEffect, onSettled } from "solid-js";
import * as monaco from "monaco-editor";
import { getOrCreateModel, setModelValueIfChanged } from "../../lib/monaco-models";

export type MonacoEditorProps = {
  value: string;
  language?: string;
  path?: string;
  theme?: string;
  options?: monaco.editor.IStandaloneEditorConstructionOptions;
  keepCurrentModel?: boolean;
  onMount?: (editor: monaco.editor.IStandaloneCodeEditor, monacoApi: typeof monaco) => void;
};

export const MonacoEditor = (props: MonacoEditorProps) => {
  let container!: HTMLDivElement;
  let editor: monaco.editor.IStandaloneCodeEditor | undefined;

  const modelUri = (path: string | undefined): monaco.Uri =>
    monaco.Uri.parse(path ? `inmemory://model/${path}` : "inmemory://model/untitled");

  const syncModel = (): void => {
    if (!editor) return;
    const uri = modelUri(props.path);
    const next = getOrCreateModel(props.value, props.language, uri);
    if (editor.getModel() !== next) {
      editor.setModel(next);
    } else {
      setModelValueIfChanged(next, props.value);
    }
  };

  onSettled(() => {
    const uri = modelUri(props.path);
    const model = getOrCreateModel(props.value, props.language, uri);
    editor = monaco.editor.create(container, {
      ...props.options,
      model,
      theme: props.theme,
      automaticLayout: props.options?.automaticLayout ?? true,
    });
    props.onMount?.(editor, monaco);

    return () => {
      const current = editor;
      editor = undefined;
      const owned = current?.getModel();
      current?.dispose();
      if (!props.keepCurrentModel) {
        owned?.dispose();
      }
    };
  });

  createEffect(
    () => [props.value, props.language, props.path] as const,
    () => {
      syncModel();
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
      if (options && editor) {
        editor.updateOptions(options);
        const model = editor.getModel();
        if (options.tabSize !== undefined) {
          model?.updateOptions({ tabSize: options.tabSize });
        }
      }
    }
  );

  return (
    <div
      ref={(element) => (container = element)}
      class="monaco-editor-host"
      style={{ width: "100%", height: "100%" }}
    />
  );
};
