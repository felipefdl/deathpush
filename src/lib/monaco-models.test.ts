import { describe, expect, it, vi } from "vite-plus/test";
import {
  applyDiffModelOptions,
  disposeOwnedModel,
  getOrCreateModel,
  replaceEditorModel,
  setModelValueIfChanged,
} from "./monaco-models";

const createModelStub = (value: string, languageId = "plaintext") => {
  const model = {
    value,
    languageId,
    getValue: () => model.value,
    setValue: vi.fn((next: string) => {
      model.value = next;
    }),
    getLanguageId: () => model.languageId,
    updateOptions: vi.fn(),
  };
  return model;
};

vi.mock("monaco-editor", () => {
  const models = new Map<string, ReturnType<typeof createModelStub>>();
  return {
    default: {},
    editor: {
      getModel: (uri: { toString: () => string }) => models.get(uri.toString()) ?? null,
      createModel: (value: string, language: string | undefined, uri: { toString: () => string }) => {
        const model = createModelStub(value, language ?? "plaintext");
        models.set(uri.toString(), model);
        return model;
      },
      setModelLanguage: (model: ReturnType<typeof createModelStub>, language: string) => {
        model.languageId = language;
      },
    },
    Uri: {
      parse: (value: string) => ({ toString: () => value }),
    },
  };
});

describe("monaco model helpers", () => {
  it("creates a model and reuses it for the same URI", async () => {
    const monaco = await import("monaco-editor");
    const uri = monaco.Uri.parse("inmemory://model/a.ts");
    const first = getOrCreateModel("one", "typescript", uri);
    const second = getOrCreateModel("two", "typescript", uri);
    expect(second).toBe(first);
    expect(first.getValue()).toBe("two");
  });

  it("updates language on an existing model", async () => {
    const monaco = await import("monaco-editor");
    const uri = monaco.Uri.parse("inmemory://model/b.ts");
    const model = getOrCreateModel("code", "typescript", uri);
    getOrCreateModel("code", "javascript", uri);
    expect(model.getLanguageId()).toBe("javascript");
  });

  it("skips setValue when the model already has the same text", () => {
    const model = createModelStub("same");
    setModelValueIfChanged(model as never, "same");
    expect(model.setValue).not.toHaveBeenCalled();
    setModelValueIfChanged(model as never, "next");
    expect(model.setValue).toHaveBeenCalledWith("next");
  });

  it("updates tab size on both diff models", () => {
    const original = createModelStub("orig");
    const modified = createModelStub("mod");
    const editor = {
      getOriginalEditor: () => ({ getModel: () => original }),
      getModifiedEditor: () => ({ getModel: () => modified }),
    };
    applyDiffModelOptions(editor as never, { tabSize: 2 });
    expect(original.updateOptions).toHaveBeenCalledWith({ tabSize: 2 });
    expect(modified.updateOptions).toHaveBeenCalledWith({ tabSize: 2 });
  });
});

describe("monaco model ownership", () => {
  const createOwnedModel = (attached = false) => {
    const model = {
      attached,
      dispose: vi.fn(() => {
        model.attached = false;
      }),
      isAttachedToEditor: () => model.attached,
    };
    return model;
  };

  const createEditor = (model: ReturnType<typeof createOwnedModel> | null) => {
    const editor = {
      current: model,
      getModel: () => editor.current as never,
      setModel: vi.fn((next: ReturnType<typeof createOwnedModel> | null) => {
        if (editor.current && editor.current !== next) {
          editor.current.attached = false;
        }
        editor.current = next;
        if (next) next.attached = true;
      }),
      dispose: vi.fn(() => {
        if (editor.current) editor.current.attached = false;
      }),
    };
    return editor;
  };

  it("disposes a detached model when it is not retained", () => {
    const model = createOwnedModel(false);
    disposeOwnedModel(model as never, false);
    expect(model.dispose).toHaveBeenCalledOnce();
  });

  it("keeps a retained or still-attached model", () => {
    const retained = createOwnedModel(false);
    disposeOwnedModel(retained as never, true);
    expect(retained.dispose).not.toHaveBeenCalled();

    const attached = createOwnedModel(true);
    disposeOwnedModel(attached as never, false);
    expect(attached.dispose).not.toHaveBeenCalled();
  });

  it("replaces the editor model and disposes the previous detached model", () => {
    const previous = createOwnedModel(true);
    const next = createOwnedModel(false);
    const editor = createEditor(previous);

    replaceEditorModel(editor as never, next as never, false);

    expect(editor.setModel).toHaveBeenCalledWith(next);
    expect(previous.attached).toBe(false);
    expect(previous.dispose).toHaveBeenCalledOnce();
    expect(next.dispose).not.toHaveBeenCalled();
  });

  it("replaces the editor model without disposing a retained previous model", () => {
    const previous = createOwnedModel(true);
    const next = createOwnedModel(false);
    const editor = createEditor(previous);

    replaceEditorModel(editor as never, next as never, true);

    expect(editor.setModel).toHaveBeenCalledWith(next);
    expect(previous.dispose).not.toHaveBeenCalled();
  });

  it("does not dispose a previous model that remains attached after replace", () => {
    const previous = createOwnedModel(true);
    const next = createOwnedModel(false);
    const editor = {
      getModel: () => previous as never,
      setModel: vi.fn(() => {
        previous.attached = true;
      }),
    };

    replaceEditorModel(editor as never, next as never, false);

    expect(editor.setModel).toHaveBeenCalledWith(next);
    expect(previous.dispose).not.toHaveBeenCalled();
  });

  it("disposes the current model on cleanup only after detach when not retained", () => {
    const model = createOwnedModel(true);
    const editor = createEditor(model);
    editor.dispose();
    disposeOwnedModel(model as never, false);
    expect(model.dispose).toHaveBeenCalledOnce();
  });

  it("keeps the current model on cleanup when retained or still attached", () => {
    const retained = createOwnedModel(true);
    const editor = createEditor(retained);
    editor.dispose();
    disposeOwnedModel(retained as never, true);
    expect(retained.dispose).not.toHaveBeenCalled();

    const shared = createOwnedModel(true);
    disposeOwnedModel(shared as never, false);
    expect(shared.dispose).not.toHaveBeenCalled();
  });
});
