import { describe, expect, it, vi } from "vite-plus/test";
import {
  applyDiffModelOptions,
  disposeOwnedDiffModels,
  disposeOwnedModel,
  getOrCreateModel,
  replaceDiffEditorModels,
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

describe("monaco diff model ownership", () => {
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

  const createDiffEditor = (
    original: ReturnType<typeof createOwnedModel> | null,
    modified: ReturnType<typeof createOwnedModel> | null
  ) => {
    const editor = {
      current: original && modified ? { original, modified } : null,
      getModel: () => editor.current as never,
      setModel: vi.fn(
        (next: { original: ReturnType<typeof createOwnedModel>; modified: ReturnType<typeof createOwnedModel> }) => {
          const previous = editor.current;
          if (previous) {
            const nextModels = new Set([next.original, next.modified]);
            if (!nextModels.has(previous.original)) previous.original.attached = false;
            if (!nextModels.has(previous.modified)) previous.modified.attached = false;
          }
          editor.current = next;
          next.original.attached = true;
          next.modified.attached = true;
        }
      ),
      dispose: vi.fn(() => {
        if (!editor.current) return;
        editor.current.original.attached = false;
        editor.current.modified.attached = false;
      }),
    };
    return editor;
  };

  it("replaces the diff pair and disposes superseded detached models", () => {
    const previousOriginal = createOwnedModel(true);
    const previousModified = createOwnedModel(true);
    const nextOriginal = createOwnedModel(false);
    const nextModified = createOwnedModel(false);
    const editor = createDiffEditor(previousOriginal, previousModified);

    replaceDiffEditorModels(
      editor as never,
      { original: nextOriginal as never, modified: nextModified as never },
      { original: false, modified: false }
    );

    expect(editor.setModel).toHaveBeenCalledWith({ original: nextOriginal, modified: nextModified });
    expect(previousOriginal.dispose).toHaveBeenCalledOnce();
    expect(previousModified.dispose).toHaveBeenCalledOnce();
    expect(nextOriginal.dispose).not.toHaveBeenCalled();
    expect(nextModified.dispose).not.toHaveBeenCalled();
  });

  it("replaces the diff pair without disposing retained sides", () => {
    const previousOriginal = createOwnedModel(true);
    const previousModified = createOwnedModel(true);
    const nextOriginal = createOwnedModel(false);
    const nextModified = createOwnedModel(false);
    const editor = createDiffEditor(previousOriginal, previousModified);

    replaceDiffEditorModels(
      editor as never,
      { original: nextOriginal as never, modified: nextModified as never },
      { original: true, modified: true }
    );

    expect(editor.setModel).toHaveBeenCalledWith({ original: nextOriginal, modified: nextModified });
    expect(previousOriginal.dispose).not.toHaveBeenCalled();
    expect(previousModified.dispose).not.toHaveBeenCalled();
  });

  it("does not dispose a previous model that remains attached after replace", () => {
    const previousOriginal = createOwnedModel(true);
    const previousModified = createOwnedModel(true);
    const nextOriginal = createOwnedModel(false);
    const nextModified = createOwnedModel(false);
    const editor = {
      getModel: () => ({ original: previousOriginal, modified: previousModified }) as never,
      setModel: vi.fn(() => {
        previousOriginal.attached = true;
        previousModified.attached = true;
      }),
    };

    replaceDiffEditorModels(
      editor as never,
      { original: nextOriginal as never, modified: nextModified as never },
      { original: false, modified: false }
    );

    expect(editor.setModel).toHaveBeenCalledWith({ original: nextOriginal, modified: nextModified });
    expect(previousOriginal.dispose).not.toHaveBeenCalled();
    expect(previousModified.dispose).not.toHaveBeenCalled();
  });

  it("does not dispose when the diff pair identity is unchanged", () => {
    const original = createOwnedModel(true);
    const modified = createOwnedModel(true);
    const editor = createDiffEditor(original, modified);

    replaceDiffEditorModels(
      editor as never,
      { original: original as never, modified: modified as never },
      { original: false, modified: false }
    );

    expect(editor.setModel).not.toHaveBeenCalled();
    expect(original.dispose).not.toHaveBeenCalled();
    expect(modified.dispose).not.toHaveBeenCalled();
  });

  it("disposes a shared original/modified model only once", () => {
    const shared = createOwnedModel(true);
    const nextOriginal = createOwnedModel(false);
    const nextModified = createOwnedModel(false);
    const editor = createDiffEditor(shared, shared);

    replaceDiffEditorModels(
      editor as never,
      { original: nextOriginal as never, modified: nextModified as never },
      { original: false, modified: false }
    );

    expect(editor.setModel).toHaveBeenCalledWith({ original: nextOriginal, modified: nextModified });
    expect(shared.dispose).toHaveBeenCalledOnce();
  });

  it("disposes detached diff models on cleanup when not retained", () => {
    const original = createOwnedModel(true);
    const modified = createOwnedModel(true);
    const editor = createDiffEditor(original, modified);
    const models = editor.getModel();
    editor.dispose();
    disposeOwnedDiffModels(models, { original: false, modified: false });
    expect(original.dispose).toHaveBeenCalledOnce();
    expect(modified.dispose).toHaveBeenCalledOnce();
  });

  it("keeps retained or still-attached diff models on cleanup", () => {
    const retainedOriginal = createOwnedModel(true);
    const retainedModified = createOwnedModel(true);
    const editor = createDiffEditor(retainedOriginal, retainedModified);
    editor.dispose();
    disposeOwnedDiffModels(editor.getModel(), { original: true, modified: true });
    expect(retainedOriginal.dispose).not.toHaveBeenCalled();
    expect(retainedModified.dispose).not.toHaveBeenCalled();

    const shared = createOwnedModel(true);
    disposeOwnedDiffModels(
      { original: shared as never, modified: shared as never },
      { original: false, modified: false }
    );
    expect(shared.dispose).not.toHaveBeenCalled();
  });

  it("disposes a shared cleanup model only once", () => {
    const shared = createOwnedModel(false);
    disposeOwnedDiffModels(
      { original: shared as never, modified: shared as never },
      { original: false, modified: false }
    );
    expect(shared.dispose).toHaveBeenCalledOnce();
  });
});
