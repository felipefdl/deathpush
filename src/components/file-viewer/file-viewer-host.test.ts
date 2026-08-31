import { cleanup, render } from "@solidjs/testing-library";
import { createEffect, flush } from "solid-js";
import { afterEach, beforeEach, describe, it, expect, vi } from "vite-plus/test";
import type { FileContent } from "../../lib/git-types";
import { explorerStore } from "../../stores/explorer-store";

const diskGuardMock = vi.hoisted(() => ({
  onReload: null as ((content: FileContent, incomingSha: string) => void) | null,
}));

vi.mock("../../hooks/use-disk-guard", () => ({
  useDiskGuard: (args: { onReload: (content: FileContent, incomingSha: string) => void }) => {
    diskGuardMock.onReload = args.onReload;
  },
}));

vi.mock("../../lib/pierre/sha", () => ({
  sha256Utf8: async () => "sha",
}));

const pierreContents: string[] = [];
const pierreRenders: Array<{ contents: string; cacheKey: string }> = [];

vi.mock("../pierre/pierre-file", () => ({
  PierreFile: (props: { contents: string; cacheKey: string }) => {
    createEffect(
      () => [props.contents, props.cacheKey] as const,
      ([contents, cacheKey]) => {
        pierreContents.push(contents);
        pierreRenders.push({ contents, cacheKey });
      }
    );
    const host = document.createElement("div");
    host.className = "pierre-file-host";
    return host;
  },
}));

import { FileViewer, isPierreHostReady } from "./file-viewer";

beforeEach(() => {
  diskGuardMock.onReload = null;
  pierreContents.length = 0;
  pierreRenders.length = 0;
  explorerStore.getState().reset();
});

afterEach(() => {
  cleanup();
  explorerStore.getState().reset();
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

  it("keeps the editor while the next file is still loading", async () => {
    const result = render(() => FileViewer());
    flush();

    explorerStore.getState().setSelectedPath("README.md");
    explorerStore.getState().setFileContent({
      path: "README.md",
      content: "# DeathPush",
      language: null,
      fileType: "text",
    });
    flush();
    await Promise.resolve();
    flush();

    explorerStore.getState().setSelectedPath("vite.config.ts");
    flush();

    expect(result.container.querySelector(".pierre-file-host")).not.toBeNull();
    expect(result.container.textContent).not.toContain("Select a file to view its contents");
  });

  it("does not show the empty prompt after a file is selected", () => {
    const result = render(() => FileViewer());
    flush();

    explorerStore.getState().setSelectedPath("vite.config.ts");
    flush();

    expect(result.container.textContent).not.toContain("Select a file to view its contents");
    expect(result.container.querySelector(".file-viewer-header")).not.toBeNull();
  });

  it("applies a second same-path content update to the editor", async () => {
    render(() => FileViewer());
    flush();

    explorerStore.getState().setSelectedPath("NOTICE");
    explorerStore.getState().setFileContent({
      path: "NOTICE",
      content: "first",
      language: null,
      fileType: "text",
    });
    flush();
    await Promise.resolve();
    flush();

    explorerStore.getState().setFileContent({
      path: "NOTICE",
      content: "second",
      language: null,
      fileType: "text",
    });
    flush();
    await Promise.resolve();
    flush();

    expect(pierreContents[pierreContents.length - 1]).toBe("second");
  });

  it("does not expose a reload cache key with the previous bytes", async () => {
    render(() => FileViewer());
    flush();

    explorerStore.getState().setSelectedPath("NOTICE");
    explorerStore.getState().setFileContent({
      path: "NOTICE",
      content: "first",
      language: null,
      fileType: "text",
    });
    flush();
    await Promise.resolve();
    flush();

    diskGuardMock.onReload?.(
      {
        path: "NOTICE",
        content: "second",
        language: null,
        fileType: "text",
      },
      "sha-second"
    );
    flush();

    diskGuardMock.onReload?.(
      {
        path: "NOTICE",
        content: "third",
        language: null,
        fileType: "text",
      },
      "sha-third"
    );
    flush();

    expect(pierreRenders).not.toContainEqual({ contents: "first", cacheKey: "NOTICE#1" });
    expect(pierreRenders).not.toContainEqual({ contents: "second", cacheKey: "NOTICE#2" });
    expect(pierreRenders[pierreRenders.length - 1]).toEqual({ contents: "third", cacheKey: "NOTICE#2" });
  });
});

describe("isPierreHostReady", () => {
  it("is false until loaded content and session share one path", () => {
    expect(isPierreHostReady({ path: "src/b.ts" }, { path: "src/a.ts" })).toBe(false);
    expect(isPierreHostReady(null, { path: "src/b.ts" })).toBe(false);
    expect(isPierreHostReady({ path: "src/b.ts" }, null)).toBe(false);
  });

  it("is true when the host can use the same loaded file", () => {
    expect(isPierreHostReady({ path: "src/b.ts" }, { path: "src/b.ts" })).toBe(true);
  });
});
