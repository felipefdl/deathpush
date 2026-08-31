import { describe, expect, it, vi } from "vite-plus/test";
import {
  ancestorDirectoryPaths,
  directoryPathCandidates,
  nextPersistedExpandedPaths,
  restoreExpandedDirectoryPaths,
  restoreSelectedFilePath,
  snapshotExpandedDirectoryPaths,
  type TreeStateItem,
  type TreeStateModel,
} from "./tree-state";

const directory = (expanded: boolean): { item: TreeStateItem; expand: ReturnType<typeof vi.fn> } => {
  const expand = vi.fn();
  return {
    expand,
    item: {
      isDirectory: () => true,
      isExpanded: () => expanded,
      expand,
      select: vi.fn(),
    },
  };
};

const file = (): { item: TreeStateItem; select: ReturnType<typeof vi.fn> } => {
  const select = vi.fn();
  return {
    select,
    item: {
      isDirectory: () => false,
      isExpanded: () => false,
      expand: vi.fn(),
      select,
    },
  };
};

describe("tree-state", () => {
  it("includes both slashed and bare directory ids", () => {
    expect(directoryPathCandidates("src/")).toEqual(["src/", "src"]);
    expect(ancestorDirectoryPaths("src/lib/trees.ts")).toEqual(["src/", "src/lib/"]);
  });

  it("snapshots expanded directories even when getItem uses a bare path", () => {
    const src = directory(true);
    const items = new Map<string, TreeStateItem>([
      ["src", src.item],
      ["src/lib/trees.ts", file().item],
    ]);
    const model: TreeStateModel = {
      getItem: (path) => items.get(path) ?? null,
      getFocusedPath: () => null,
      getSelectedPaths: () => [],
      focusPath: vi.fn(),
    };

    expect(snapshotExpandedDirectoryPaths(model, ["src/lib/trees.ts"])).toEqual(["src/"]);
  });

  it("restores expansion and the selected file", () => {
    const src = directory(false);
    const readme = file();
    const focusPath = vi.fn();
    const items = new Map<string, TreeStateItem>([
      ["src/", src.item],
      ["README.md", readme.item],
    ]);
    const model: TreeStateModel = {
      getItem: (path) => items.get(path) ?? null,
      getFocusedPath: () => null,
      getSelectedPaths: () => [],
      focusPath,
    };

    restoreExpandedDirectoryPaths(model, ["src/"]);
    restoreSelectedFilePath(model, "README.md");

    expect(src.expand).toHaveBeenCalledTimes(1);
    expect(readme.select).toHaveBeenCalledTimes(1);
    expect(focusPath).toHaveBeenCalledWith("README.md");
  });

  it("does not replace stored expansion with an empty snapshot", () => {
    expect(nextPersistedExpandedPaths(["src/", "src/lib/"], [])).toEqual(["src/", "src/lib/"]);
    expect(nextPersistedExpandedPaths(["src/"], ["src/", "src/lib/"])).toEqual(["src/", "src/lib/"]);
  });
});
