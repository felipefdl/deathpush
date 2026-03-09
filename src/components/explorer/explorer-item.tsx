import { useCallback, useState } from "react";
import type { ExplorerEntry } from "../../lib/git-types";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import { useExplorerStore } from "../../stores/explorer-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import * as commands from "../../lib/tauri-commands";

interface ExplorerItemProps {
  entry: ExplorerEntry;
  depth: number;
  onToggleDir: (path: string) => void;
  expanded?: boolean;
}

export const ExplorerItem = ({ entry, depth, onToggleDir, expanded }: ExplorerItemProps) => {
  const selectedPath = useExplorerStore((s) => s.selectedPath);
  const setSelectedPath = useExplorerStore((s) => s.setSelectedPath);
  const setFileContent = useExplorerStore((s) => s.setFileContent);
  const setError = useRepositoryStore((s) => s.setError);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const isSelected = !entry.isDirectory && selectedPath === entry.path;

  const handleClick = useCallback(() => {
    if (entry.isDirectory) {
      onToggleDir(entry.path);
      return;
    }
    setSelectedPath(entry.path);
    useLayoutStore.getState().setMainView("file");
    commands.readFileContent(entry.path).then(setFileContent).catch((err) => setError(String(err)));
  }, [entry.path, entry.isDirectory, onToggleDir, setSelectedPath, setFileContent, setError]);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const handleOpenInEditor = useCallback(async () => {
    try {
      await commands.openInEditor(entry.path);
    } catch (err) {
      setError(String(err));
    }
  }, [entry.path, setError]);

  const handleRevealInFinder = useCallback(async () => {
    try {
      await commands.revealInFileManager(entry.path);
    } catch (err) {
      setError(String(err));
    }
  }, [entry.path, setError]);

  const handleCopyPath = useCallback(async () => {
    const root = useRepositoryStore.getState().status?.root ?? "";
    const fullPath = root ? `${root}/${entry.path}` : entry.path;
    await navigator.clipboard.writeText(fullPath);
  }, [entry.path]);

  const handleCopyRelativePath = useCallback(async () => {
    await navigator.clipboard.writeText(entry.path);
  }, [entry.path]);

  const getContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [];
    if (!entry.isDirectory) {
      items.push({ label: "Open in Editor", icon: "go-to-file", action: handleOpenInEditor });
      items.push({ label: "", action: () => {}, separator: true });
    }
    items.push(
      { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder },
      { label: "", action: () => {}, separator: true },
      { label: "Copy Path", icon: "copy", action: handleCopyPath },
      { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
    );
    return items;
  };

  const iconClasses = entry.isDirectory
    ? getFileIconClasses(entry.name, "folder")
    : getFileIconClasses(entry.path, "file");

  return (
    <>
      <div
        className={`explorer-item${isSelected ? " selected" : ""}`}
        style={{ paddingLeft: 12 + depth * 12 }}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
      >
        {entry.isDirectory && (
          <span className={`codicon codicon-chevron-down resource-group-chevron${expanded ? "" : " collapsed"}`} />
        )}
        <span className={`resource-item-icon ${iconClasses}`} />
        <span className="explorer-item-name">{entry.name}</span>
        {!entry.isDirectory && (
          <div className="explorer-item-actions">
            <button className="inline-action" onClick={(e) => { e.stopPropagation(); handleOpenInEditor(); }} title="Open in Editor">
              <span className="codicon codicon-go-to-file" />
            </button>
          </div>
        )}
      </div>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getContextMenuItems()}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
};
