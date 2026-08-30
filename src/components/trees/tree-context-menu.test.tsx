import { cleanup, fireEvent } from "@solidjs/testing-library";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { renderTreeContextMenu } from "./tree-context-menu";

describe("renderTreeContextMenu", () => {
  afterEach(cleanup);

  it("runs the selected command and closes the Trees menu", () => {
    const action = vi.fn();
    const close = vi.fn();
    const host = renderTreeContextMenu([{ label: "Open", action }], {
      anchorElement: document.body,
      anchorRect: { top: 10, right: 30, bottom: 30, left: 10, width: 20, height: 20, x: 10, y: 10 },
      close,
      restoreFocus: vi.fn(),
    });

    expect(host.dataset.fileTreeContextMenuRoot).toBe("true");
    fireEvent.click(document.querySelector(".context-menu-item")!);
    expect(action).toHaveBeenCalledTimes(1);
    expect(close).toHaveBeenCalledWith({ restoreFocus: true });
  });
});
