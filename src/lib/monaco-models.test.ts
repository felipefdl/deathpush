import { describe, expect, it, vi } from "vite-plus/test";
import { applyDiffModelOptions, getOrCreateModel, setModelValueIfChanged } from "./monaco-models";

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
