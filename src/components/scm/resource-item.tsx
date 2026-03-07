import { useCallback, useEffect, useRef, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry, ResourceGroupKind } from "../../lib/git-types";
import { getStatusColor } from "../../lib/status-colors";
import { getStatusLabel } from "../../lib/status-icons";
import { useRepositoryStore } from "../../stores/repository-store";
import { useDiff } from "../../hooks/use-diff";
import * as commands from "../../lib/tauri-commands";
import { ContextMenu, type ContextMenuItem } from "./context-menu";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import { useLayoutStore } from "../../stores/layout-store";

interface ResourceItemProps {
  file: FileEntry;
  groupKind: ResourceGroupKind;
  focused?: boolean;
}

export const ResourceItem = ({ file, groupKind, focused }: ResourceItemProps) => {
  const {
    selectedFile, setStatus, setError, selectedFiles, toggleFileSelection,
    clearFileSelection, startOperation, endOperation, isDiffDirty,
  } = useRepositoryStore();
  const { loadDiff } = useDiff();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const itemRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (focused && itemRef.current) {
      itemRef.current.scrollIntoView({ block: "nearest" });
    }
  }, [focused]);

  const isStaged = groupKind === "index";
  const selectionKey = `${isStaged ? "staged" : "unstaged"}:${file.path}`;
  const isSelected = selectedFile?.path === file.path && selectedFile?.staged === isStaged;
  const isMultiSelected = selectedFiles.has(selectionKey);
  const fileName = file.path.split("/").pop() ?? file.path;
  const dirPath = file.path.includes("/") ? file.path.substring(0, file.path.lastIndexOf("/")) : "";
  const color = getStatusColor(file.status);
  const label = getStatusLabel(file.status);
  const isDeleted = file.status === "deleted" || file.status === "indexDeleted"
    || file.status === "bothDeleted" || file.status === "deletedByThem" || file.status === "deletedByUs";

  const getSelectedPaths = useCallback((prefix: string): string[] => {
    const paths: string[] = [];
    for (const key of selectedFiles) {
      if (key.startsWith(prefix + ":")) {
        paths.push(key.substring(prefix.length + 1));
      }
    }
    return paths;
  }, [selectedFiles]);

  const handleClick = useCallback((e: React.MouseEvent) => {
    if (e.ctrlKey || e.metaKey) {
      toggleFileSelection(selectionKey, true, false);
      return;
    }
    if (e.shiftKey) {
      toggleFileSelection(selectionKey, false, true);
      return;
    }
    clearFileSelection();
    loadDiff(file.path, isStaged);
    const { mainView, setMainView } = useLayoutStore.getState();
    if (mainView !== "changes") setMainView("changes");
  }, [file.path, isStaged, loadDiff, selectionKey, toggleFileSelection, clearFileSelection]);

  const handleStage = useCallback(async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    startOperation("stage");
    try {
      const paths = selectedFiles.size > 1 && isMultiSelected
        ? getSelectedPaths("unstaged")
        : [file.path];
      if (paths.length === 0) paths.push(file.path);
      const status = await commands.stageFiles(paths);
      setStatus(status);
      clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  }, [file.path, setStatus, setError, startOperation, endOperation, isMultiSelected, selectedFiles, getSelectedPaths, clearFileSelection]);

  const handleUnstage = useCallback(async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    startOperation("unstage");
    try {
      const paths = selectedFiles.size > 1 && isMultiSelected
        ? getSelectedPaths("staged")
        : [file.path];
      if (paths.length === 0) paths.push(file.path);
      const status = await commands.unstageFiles(paths);
      setStatus(status);
      clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  }, [file.path, setStatus, setError, startOperation, endOperation, isMultiSelected, selectedFiles, getSelectedPaths, clearFileSelection]);

  const handleDiscard = useCallback(async (e?: React.MouseEvent) => {
    e?.stopPropagation();
    const paths = selectedFiles.size > 1 && isMultiSelected
      ? getSelectedPaths("unstaged")
      : [file.path];
    if (paths.length === 0) paths.push(file.path);
    const msg = paths.length > 1
      ? `Are you sure you want to discard changes in ${paths.length} file(s)?\n\nThis action is irreversible.`
      : `Are you sure you want to discard changes in "${file.path.split("/").pop()}"?\n\nThis action is irreversible.`;
    const confirmed = await confirm(msg, {
      title: "Discard Changes",
      kind: "warning",
      okLabel: "Discard",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    startOperation("discard");
    try {
      const status = await commands.discardChanges(paths);
      setStatus(status);
      clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  }, [file.path, setStatus, setError, startOperation, endOperation, isMultiSelected, selectedFiles, getSelectedPaths, clearFileSelection]);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const handleOpenFile = useCallback(async () => {
    try {
      await commands.openInEditor(file.path);
    } catch (err) {
      setError(String(err));
    }
  }, [file.path, setError]);

  const handleRevealInFinder = useCallback(async () => {
    try {
      await commands.revealInFileManager(file.path);
    } catch (err) {
      setError(String(err));
    }
  }, [file.path, setError]);

  const handleCopyPath = useCallback(async () => {
    const root = useRepositoryStore.getState().status?.root ?? "";
    const fullPath = root ? `${root}/${file.path}` : file.path;
    await navigator.clipboard.writeText(fullPath);
  }, [file.path]);

  const handleCopyRelativePath = useCallback(async () => {
    await navigator.clipboard.writeText(file.path);
  }, [file.path]);

  const handleDeleteFile = useCallback(async () => {
    const confirmed = await confirm(
      `Are you sure you want to move "${file.path.split("/").pop()}" to the trash?`,
      { title: "Move to Trash", kind: "warning", okLabel: "Move to Trash", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    startOperation("delete");
    try {
      const status = await commands.deleteFile(file.path);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("delete");
    }
  }, [file.path, setStatus, setError, startOperation, endOperation]);

  const handleShowFileHistory = useCallback(() => {
    useLayoutStore.getState().setMainView("history");
    window.dispatchEvent(
      new CustomEvent("deathpush:file-history", { detail: { path: file.path } }),
    );
  }, [file.path]);

  const handleAddToGitignore = useCallback(async () => {
    try {
      const status = await commands.addToGitignore(file.path);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [file.path, setStatus, setError]);

  const getContextMenuItems = (): ContextMenuItem[] => {
    if (isStaged) {
      return [
        { label: "Open Changes", icon: "diff", action: () => loadDiff(file.path, isStaged) },
        { label: "Open File", icon: "go-to-file", action: handleOpenFile },
        { label: "Show File History", icon: "history", action: handleShowFileHistory },
        { label: "", action: () => {}, separator: true },
        { label: "Unstage Changes", icon: "remove", action: () => handleUnstage() },
        { label: "", action: () => {}, separator: true },
        { label: "Copy Path", icon: "copy", action: handleCopyPath },
        { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
        { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder },
      ];
    }
    const items: ContextMenuItem[] = [
      { label: "Open Changes", icon: "diff", action: () => loadDiff(file.path, isStaged) },
      { label: "Open File", icon: "go-to-file", action: handleOpenFile },
      { label: "Show File History", icon: "history", action: handleShowFileHistory },
      { label: "", action: () => {}, separator: true },
      { label: "Stage Changes", icon: "add", action: () => handleStage() },
    ];
    if (file.status !== "untracked") {
      items.push({ label: "Discard Changes", icon: "discard", action: () => handleDiscard() });
    }
    items.push(
      { label: "", action: () => {}, separator: true },
      { label: "Copy Path", icon: "copy", action: handleCopyPath },
      { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
      { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder },
    );
    if (!isDeleted) {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Move to Trash", icon: "trash", action: handleDeleteFile },
      );
    }
    if (file.status === "untracked") {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Add to .gitignore", icon: "exclude", action: handleAddToGitignore },
      );
    }
    return items;
  };

  const className = [
    "resource-item",
    isSelected ? "selected" : "",
    isMultiSelected ? "multi-selected" : "",
    focused ? "focused" : "",
  ].filter(Boolean).join(" ");

  return (
    <>
      <div
        ref={itemRef}
        className={className}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        style={{ color }}
      >
        <span className={`resource-item-icon ${getFileIconClasses(file.path, "file")}`} />
        <span className={`resource-item-name${isDeleted ? " resource-item-deleted" : ""}`}>
          {fileName}
          {isSelected && isDiffDirty && <span className="diff-header-dirty"> *</span>}
        </span>
        {dirPath && <span className={`resource-item-path${isDeleted ? " resource-item-deleted" : ""}`}>{dirPath}</span>}
        <span className="resource-item-spacer" />
        <div className="resource-item-actions">
          {isStaged ? (
            <button className="inline-action" onClick={(e) => handleUnstage(e)} title="Unstage">
              <span className="codicon codicon-remove" />
            </button>
          ) : (
            <>
              {file.status !== "untracked" && (
                <button className="inline-action" onClick={(e) => handleDiscard(e)} title="Discard Changes">
                  <span className="codicon codicon-discard" />
                </button>
              )}
              <button className="inline-action" onClick={(e) => handleStage(e)} title="Stage Changes">
                <span className="codicon codicon-add" />
              </button>
            </>
          )}
        </div>
        <span className="resource-item-status" style={{ color }}>{label}</span>
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
