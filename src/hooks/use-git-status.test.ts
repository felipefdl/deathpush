import { describe, it, expect, vi } from "vite-plus/test";

const { pierreHostModuleLoaded } = vi.hoisted(() => ({
  pierreHostModuleLoaded: vi.fn(),
}));

vi.mock("../components/pierre/pierre-file-diff", () => {
  pierreHostModuleLoaded();
  return { getScmSession: () => null };
});

import { createScmDiskGuard, isScmWatcherTarget, runScmDiskGuard } from "./use-git-status";
import type { SaveSession } from "../lib/pierre/save-session";
import type { DiffContent } from "../lib/git-types";
import type { SelectedFile } from "../stores/repository-store";

const session = (overrides: Partial<SaveSession> = {}): SaveSession => ({
  path: "src/a.ts",
  diskSha: "aaa",
  pendingSha: null,
  cacheGeneration: 0,
  ...overrides,
});

const selected = (overrides: Partial<SelectedFile> = {}): SelectedFile => ({
  path: "src/a.ts",
  staged: false,
  groupKind: "workingTree",
  ...overrides,
});

const diff = (modified: string): DiffContent => ({
  path: "src/a.ts",
  original: "",
  modified,
  originalLanguage: "typescript",
  fileType: "text",
});

describe("boot graph", () => {
  it("does not load the Pierre diff host through repository status", () => {
    expect(pierreHostModuleLoaded).not.toHaveBeenCalled();
  });
});
describe("isScmWatcherTarget", () => {
  it("watches working-tree, untracked, and index files", () => {
    expect(isScmWatcherTarget(selected())).toBe(true);
    expect(isScmWatcherTarget(selected({ groupKind: "untracked" }))).toBe(true);
    expect(isScmWatcherTarget(selected({ groupKind: "index", staged: true }))).toBe(true);
  });

  it("ignores merge and an empty selection", () => {
    expect(isScmWatcherTarget(selected({ groupKind: "merge" }))).toBe(false);
    expect(isScmWatcherTarget(null)).toBe(false);
  });
});

describe("runScmDiskGuard", () => {
  it("ignores when pendingSha is set on a working-tree file", async () => {
    const getFileDiff = vi.fn();
    const onReload = vi.fn();

    await runScmDiskGuard({
      selectedFile: selected(),
      session: session({ pendingSha: "pending" }),
      getFileDiff,
      sha256Utf8: async () => "bbb",
      onReload,
    });

    expect(getFileDiff).not.toHaveBeenCalled();
    expect(onReload).not.toHaveBeenCalled();
  });

  it("ignores merge selections", async () => {
    const getFileDiff = vi.fn();
    const onReload = vi.fn();

    await runScmDiskGuard({
      selectedFile: selected({ groupKind: "merge" }),
      session: session(),
      getFileDiff,
      sha256Utf8: async () => "bbb",
      onReload,
    });

    expect(getFileDiff).not.toHaveBeenCalled();
    expect(onReload).not.toHaveBeenCalled();
  });

  it("reloads a staged selection when the index hash differs", async () => {
    const onReload = vi.fn();
    const next = diff("staged-changed");

    await runScmDiskGuard({
      selectedFile: selected({ groupKind: "index", staged: true }),
      session: session(),
      getFileDiff: async () => next,
      sha256Utf8: async () => "ccc",
      onReload,
    });

    expect(onReload).toHaveBeenCalledWith(next, "ccc");
  });

  it("reloads a staged selection even when pendingSha is set", async () => {
    const onReload = vi.fn();
    const next = diff("staged-changed");

    await runScmDiskGuard({
      selectedFile: selected({ groupKind: "index", staged: true }),
      session: session({ pendingSha: "pending" }),
      getFileDiff: async () => next,
      sha256Utf8: async () => "ccc",
      onReload,
    });

    expect(onReload).toHaveBeenCalledWith(next, "ccc");
  });

  it("reloads when getFileDiff.modified hashes differently", async () => {
    const onReload = vi.fn();
    const next = diff("changed");

    await runScmDiskGuard({
      selectedFile: selected(),
      session: session(),
      getFileDiff: async () => next,
      sha256Utf8: async () => "ccc",
      onReload,
    });

    expect(onReload).toHaveBeenCalledWith(next, "ccc");
  });

  it("discards a stale check that finishes after a newer selection", async () => {
    const run = createScmDiskGuard();
    const onReload = vi.fn();
    let finishOld: (next: DiffContent) => void = () => undefined;
    const next = diff("new");

    const old = run({
      selectedFile: selected(),
      session: session(),
      getFileDiff: () =>
        new Promise<DiffContent>((resolve) => {
          finishOld = resolve;
        }),
      sha256Utf8: async () => "old-sha",
      onReload,
    });

    await run({
      selectedFile: selected({ path: "src/b.ts" }),
      session: session({ path: "src/b.ts" }),
      getFileDiff: async () => ({ ...next, path: "src/b.ts" }),
      sha256Utf8: async () => "new-sha",
      onReload,
    });

    expect(onReload).toHaveBeenCalledTimes(1);
    expect(onReload).toHaveBeenCalledWith({ ...next, path: "src/b.ts" }, "new-sha");
    onReload.mockClear();

    finishOld(diff("old"));
    await old;
    expect(onReload).not.toHaveBeenCalled();
  });
});
