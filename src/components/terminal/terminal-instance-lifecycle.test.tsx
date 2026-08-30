import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";

const invokeMock = vi.hoisted(() =>
  vi.fn(async (command: string) => {
    if (command === "terminal_spawn") return { id: 1, shell: "zsh" };
    if (command === "terminal_foreground_process") return "zsh";
    return undefined;
  })
);

vi.mock("@wterm/dom", () => ({
  WTerm: class {
    cols = 80;
    rows = 24;
    constructor() {}
    async init() {
      return this;
    }
    write() {}
    focus() {}
    destroy() {}
    resize() {}
  },
}));

vi.mock("@wterm/ghostty", () => ({
  GhosttyCore: {
    load: vi.fn(async () => ({})),
  },
}));

vi.mock("@wterm/dom/src/terminal.css", () => ({}));

vi.mock("@tauri-apps/api/core", () => ({
  invoke: invokeMock,
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    listen: vi.fn(async () => () => undefined),
  }),
}));

vi.stubGlobal(
  "ResizeObserver",
  class {
    observe() {}
    disconnect() {}
  }
);

Object.defineProperty(document, "fonts", {
  configurable: true,
  value: { load: vi.fn(async () => []) },
});

import { TerminalInstance } from "./terminal-instance";

afterEach(() => {
  cleanup();
  invokeMock.mockClear();
  Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => 0 });
  Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 0 });
});

describe("TerminalInstance spawn", () => {
  it("spawns a session when the container is already visible", async () => {
    Object.defineProperty(HTMLElement.prototype, "clientWidth", { configurable: true, get: () => 800 });
    Object.defineProperty(HTMLElement.prototype, "clientHeight", { configurable: true, get: () => 400 });

    render(() => <TerminalInstance paneId={1} isActive={true} />);

    await waitFor(() => expect(invokeMock).toHaveBeenCalledWith("terminal_spawn", expect.anything()));
  });
});
