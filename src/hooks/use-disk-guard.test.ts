import { describe, it, expect, vi } from "vite-plus/test";
import { createFileViewerDiskGuard, runFileViewerDiskGuard } from "./use-disk-guard";
import type { SaveSession } from "../lib/pierre/save-session";
import type { FileContent } from "../lib/git-types";

const session = (overrides: Partial<SaveSession> = {}): SaveSession => ({
  path: "src/a.ts",
  diskSha: "aaa",
  pendingSha: null,
  cacheGeneration: 0,
  ...overrides,
});

const textFile = (content: string): FileContent => ({
  path: "src/a.ts",
  content,
  language: "typescript",
  fileType: "text",
});

describe("runFileViewerDiskGuard", () => {
  it("does nothing when no explorer path is selected", async () => {
    const readFileContent = vi.fn();
    const onReload = vi.fn();

    await runFileViewerDiskGuard({
      selectedPath: null,
      session: session(),
      readFileContent,
      sha256Utf8: async () => "bbb",
      onReload,
    });

    expect(readFileContent).not.toHaveBeenCalled();
    expect(onReload).not.toHaveBeenCalled();
  });

  it("ignores disk events while a write is in flight", async () => {
    const readFileContent = vi.fn();
    const onReload = vi.fn();

    await runFileViewerDiskGuard({
      selectedPath: "src/a.ts",
      session: session({ pendingSha: "pending" }),
      readFileContent,
      sha256Utf8: async () => "bbb",
      onReload,
    });

    expect(readFileContent).not.toHaveBeenCalled();
    expect(onReload).not.toHaveBeenCalled();
  });

  it("ignores when the incoming hash matches diskSha", async () => {
    const onReload = vi.fn();

    await runFileViewerDiskGuard({
      selectedPath: "src/a.ts",
      session: session(),
      readFileContent: async () => textFile("same"),
      sha256Utf8: async () => "aaa",
      onReload,
    });

    expect(onReload).not.toHaveBeenCalled();
  });

  it("reloads when disk bytes differ and no write is in flight", async () => {
    const onReload = vi.fn();
    const content = textFile("changed");

    await runFileViewerDiskGuard({
      selectedPath: "src/a.ts",
      session: session(),
      readFileContent: async () => content,
      sha256Utf8: async () => "ccc",
      onReload,
    });

    expect(onReload).toHaveBeenCalledWith(content, "ccc");
  });

  it("discards a stale check that finishes after a newer reload", async () => {
    const run = createFileViewerDiskGuard();
    const current = session();
    const onReload = vi.fn((content: FileContent, incomingSha: string) => {
      current.diskSha = incomingSha;
      void content;
    });
    let finishOld: (content: FileContent) => void = () => undefined;
    const newContent = textFile("new");

    const old = run({
      selectedPath: "src/a.ts",
      session: current,
      readFileContent: () =>
        new Promise<FileContent>((resolve) => {
          finishOld = resolve;
        }),
      sha256Utf8: async () => "old-sha",
      onReload,
    });

    await run({
      selectedPath: "src/a.ts",
      session: current,
      readFileContent: async () => newContent,
      sha256Utf8: async () => "new-sha",
      onReload,
    });

    expect(onReload).toHaveBeenCalledTimes(1);
    expect(onReload).toHaveBeenCalledWith(newContent, "new-sha");
    onReload.mockClear();

    finishOld(textFile("old"));
    await old;
    expect(onReload).not.toHaveBeenCalled();
  });
});
