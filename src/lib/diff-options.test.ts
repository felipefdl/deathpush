import { describe, it, expect, vi } from "vite-plus/test";

vi.mock("monaco-editor", () => ({
  editor: {
    ShowLightbulbIconMode: {
      Off: "off",
      OnCode: "onCode",
      On: "on",
    },
  },
}));

import { editor as MonacoEditor } from "monaco-editor";
import { buildDiffOptions, buildDiffModelOptions } from "./diff-options";
import type { EditorSettings } from "../stores/settings-store";

const mockEditor: EditorSettings = {
  fontSize: 14,
  fontFamily: "monospace",
  lineHeight: 20,
  tabSize: 2,
  wordWrap: "on",
  renderWhitespace: "none",
};

describe("buildDiffOptions", () => {
  it("sets renderSideBySide true for sideBySide mode", () => {
    const opts = buildDiffOptions(mockEditor, "sideBySide");
    expect(opts.renderSideBySide).toBe(true);
  });

  it("sets renderSideBySide false for inline mode", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect(opts.renderSideBySide).toBe(false);
  });

  it("passes editor fontSize", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect(opts.fontSize).toBe(14);
  });

  it("passes editor fontFamily", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect(opts.fontFamily).toBe("monospace");
  });

  it("passes editor wordWrap", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect(opts.wordWrap).toBe("on");
  });

  it("has correct static options", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect(opts.minimap.enabled).toBe(false);
    expect(opts.scrollBeyondLastLine).toBe(false);
    expect(opts.originalEditable).toBe(false);
    expect(opts.codeLens).toBe(false);
    expect(opts.hover.enabled).toBe("off");
    expect(opts.lightbulb.enabled).toBe(MonacoEditor.ShowLightbulbIconMode.Off);
  });

  it("omits tabSize from construction options", () => {
    const opts = buildDiffOptions(mockEditor, "inline");
    expect("tabSize" in opts).toBe(false);
  });
});

describe("buildDiffModelOptions", () => {
  it("maps editor tabSize onto the model update options", () => {
    expect(buildDiffModelOptions(mockEditor)).toEqual({ tabSize: 2 });
  });

  it("uses the configured tabSize", () => {
    expect(buildDiffModelOptions({ ...mockEditor, tabSize: 8 })).toEqual({ tabSize: 8 });
  });
});
