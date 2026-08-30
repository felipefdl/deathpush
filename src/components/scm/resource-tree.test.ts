import { describe, expect, it } from "vite-plus/test";
import type { FileEntry } from "../../lib/git-types";
import { resolveResourcePaths } from "./resource-tree";

const files: FileEntry[] = [
  { path: "README.md", status: "modified", renamePath: null },
  { path: "src/index.ts", status: "indexModified", renamePath: null },
  { path: "src/view.ts", status: "untracked", renamePath: null },
];

describe("resolveResourcePaths", () => {
  it("expands selected directories into their changed files", () => {
    expect(resolveResourcePaths(files, ["src/", "README.md"])).toEqual(["src/index.ts", "src/view.ts", "README.md"]);
  });
});
