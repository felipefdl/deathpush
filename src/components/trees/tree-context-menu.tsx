import type { ContextMenuOpenContext } from "@pierre/trees";
import { render } from "@solidjs/web";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";

export const renderTreeContextMenu = (items: ContextMenuItem[], context: ContextMenuOpenContext): HTMLElement => {
  const host = document.createElement("div");
  host.dataset.fileTreeContextMenuRoot = "true";
  let dispose = (): void => {};
  const close = (): void => {
    dispose();
    host.remove();
    context.close({ restoreFocus: true });
  };
  dispose = render(
    () => (
      <ContextMenu x={context.anchorRect.x} y={context.anchorRect.y} items={items} onClose={close} treeContextRoot />
    ),
    host
  );
  return host;
};
