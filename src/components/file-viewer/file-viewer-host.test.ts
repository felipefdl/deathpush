import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, it, expect, vi } from "vite-plus/test";
import { explorerStore } from "../../stores/explorer-store";

vi.mock("../../hooks/use-disk-guard", () => ({
  useDiskGuard: () => undefined,
}));

vi.mock("../../lib/pierre/sha", () => ({
  sha256Utf8: async () => "sha",
}));

vi.mock("../pierre/pierre-file", () => ({
  PierreFile: () => {
    const host = document.createElement("div");
    host.className = "pierre-file-host";
    return host;
  },
}));

import { FileViewer, isPierreHostReady } from "./file-viewer";

beforeEach(() => {
  explorerStore.getState().clearCache();
});

afterEach(() => {
  cleanup();
  explorerStore.getState().clearCache();
});

describe("FileViewer", () => {
  it("mounts the editor after the first matching file load", async () => {
    const result = render(() => FileViewer());
    flush();

    explorerStore.getState().setSelectedPath("NOTICE");
    explorerStore.getState().setFileContent({
      path: "NOTICE",
      content: "DeathPush",
      language: null,
      fileType: "text",
    });
    flush();
    await Promise.resolve();
    flush();

    expect(result.container.querySelector(".pierre-file-host")).not.toBeNull();
  });
});

describe("isPierreHostReady", () => {
  it("is false until selected path, loaded content, and session share one path", () => {
    expect(isPierreHostReady("src/b.ts", { path: "src/a.ts" }, { path: "src/a.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, { path: "src/a.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", null, { path: "src/b.ts" })).toBe(false);
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, null)).toBe(false);
  });

  it("is true only when the host can use the same loaded file", () => {
    expect(isPierreHostReady("src/b.ts", { path: "src/b.ts" }, { path: "src/b.ts" })).toBe(true);
  });
});
