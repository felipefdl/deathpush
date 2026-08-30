import { cleanup, render } from "@solidjs/testing-library";
import { afterEach, describe, expect, it } from "vite-plus/test";
import { ContextMenu } from "./context-menu";

describe("ContextMenu", () => {
  afterEach(cleanup);

  it("marks portaled tree menus for Trees focus handling", () => {
    render(() => <ContextMenu x={10} y={10} items={[]} onClose={() => {}} treeContextRoot />);

    expect(document.querySelector("[data-file-tree-context-menu-root]")).toBeTruthy();
  });
});
