import { explorerStore } from "../stores/explorer-store";
import { layoutStore } from "../stores/layout-store";

export const fileTreeClickedFilePath = (event: Event): string | null => {
  const target = event.target;
  if (!(target instanceof Element)) return null;
  const row = target.closest("[data-type=item]");
  if (!(row instanceof HTMLElement) || row.dataset.itemType !== "file") return null;
  return row.dataset.itemPath ?? null;
};

export const dockTerminalIfCurrentFile = (path: string): void => {
  if (explorerStore.getState().selectedPath !== path) return;
  const layout = layoutStore.getState();
  layout.dockTerminal();
  layout.setMainView("file");
};

export const shouldReloadOpenFile = (selectedPath: string | null, path: string): boolean => selectedPath !== path;
