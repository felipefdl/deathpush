import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { explorerStore } from "../stores/explorer-store";
import { repositoryStore } from "../stores/repository-store";
import { useKeyboardShortcuts } from "./use-keyboard-shortcuts";

const { sendIntentMock, isPierreFindHostOpenMock } = vi.hoisted(() => ({
  sendIntentMock: vi.fn(),
  isPierreFindHostOpenMock: vi.fn(() => false),
}));

vi.mock("../lib/session-client", () => ({
  sendIntent: sendIntentMock,
  sendDestructiveIntent: vi.fn(),
}));

vi.mock("../lib/pierre/find-host", () => ({
  isPierreFindHostOpen: isPierreFindHostOpenMock,
}));

vi.mock("../lib/tauri-commands", () => ({
  fuzzyFindFiles: vi.fn(),
  gitGrep: vi.fn(),
}));

const Shortcuts = () => {
  useKeyboardShortcuts();
  return null;
};

const pressEscape = (): void => {
  document.body.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
};

describe("useKeyboardShortcuts Escape", () => {
  beforeEach(() => {
    sendIntentMock.mockReset();
    sendIntentMock.mockResolvedValue({ kind: "ack", sessionGeneration: 0, sessionRevision: 2 });
    isPierreFindHostOpenMock.mockReturnValue(false);
    repositoryStore.setState({
      selectedFile: { path: "a.ts", staged: false, groupKind: "workingTree" },
      diff: { path: "a.ts", original: "old", modified: "new", originalLanguage: null, fileType: "text" },
      sessionGeneration: 0,
      sessionRevision: 1,
    });
    explorerStore.setState({
      selectedPath: "README.md",
      fileContent: { path: "README.md", content: "hi", language: "markdown", fileType: "text", contentHash: "hash:hi" },
    });
  });

  afterEach(() => {
    cleanup();
  });

  it("sends clearFile on Escape deselect", () => {
    render(() => <Shortcuts />);
    flush();
    pressEscape();
    expect(sendIntentMock).toHaveBeenCalledWith({ type: "clearFile" });
  });

  it("does not only clear Zustand on Escape", () => {
    render(() => <Shortcuts />);
    flush();
    pressEscape();
    expect(repositoryStore.getState().selectedFile).toBeNull();
    expect(sendIntentMock).toHaveBeenCalledTimes(1);
    expect(sendIntentMock).toHaveBeenCalledWith({ type: "clearFile" });
  });

  it("still clears explorer selection locally on Escape", () => {
    render(() => <Shortcuts />);
    flush();
    pressEscape();
    expect(explorerStore.getState().selectedPath).toBeNull();
    expect(explorerStore.getState().fileContent).toBeNull();
  });

  it("does not send clearFile when Pierre find-host is open", () => {
    isPierreFindHostOpenMock.mockReturnValue(true);
    render(() => <Shortcuts />);
    flush();
    pressEscape();
    expect(sendIntentMock).not.toHaveBeenCalled();
    expect(repositoryStore.getState().selectedFile?.path).toBe("a.ts");
  });
});
