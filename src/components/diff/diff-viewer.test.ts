import { describe, expect, it } from "vite-plus/test";
import { shouldMountTextPierre } from "./diff-viewer";

describe("shouldMountTextPierre", () => {
  it("mounts only when the stored diff belongs to the selected text load", () => {
    expect(shouldMountTextPierre({ path: "src/a.ts", groupKind: "workingTree" }, 1, { path: "src/a.ts" }, 1)).toBe(
      true
    );
    expect(shouldMountTextPierre({ path: "src/b.ts", groupKind: "workingTree" }, 2, { path: "src/a.ts" }, 1)).toBe(
      false
    );
    expect(shouldMountTextPierre({ path: "src/a.ts", groupKind: "workingTree" }, 2, { path: "src/a.ts" }, 1)).toBe(
      false
    );
    expect(shouldMountTextPierre({ path: "src/a.ts", groupKind: "merge" }, 1, { path: "src/a.ts" }, 1)).toBe(false);
    expect(shouldMountTextPierre({ path: "src/a.ts", groupKind: "workingTree" }, 1, null, 1)).toBe(false);
  });
});
