import { describe, it, expect, vi } from "vite-plus/test";
import { runFileViewerDiskGuard } from "./use-disk-guard";
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
});
