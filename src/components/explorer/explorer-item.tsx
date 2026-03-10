import { useCallback, useContext, useEffect, useRef, useState } from "react";
import { ask, confirm } from "@tauri-apps/plugin-dialog";
import type { ExplorerEntry } from "../../lib/git-types";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import type { ConflictResolution } from "../../lib/tauri-commands";
import { useExplorerStore } from "../../stores/explorer-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import { GitDecorationContext } from "./explorer-tree";
import * as commands from "../../lib/tauri-commands";

const isAlreadyExistsError = (err: unknown): boolean =>
  typeof err === "string" && err.includes("already exists");

const askConflictResolution = async (): Promise<ConflictResolution | null> => {
  const replace = await ask("A file with this name already exists. Do you want to replace it?", {
    title: "File Conflict",
    kind: "warning",
  });
  if (replace) return "replace";
  const keepBoth = await ask("Keep both files? A copy will be created with a new name.", {
    title: "File Conflict",
    kind: "info",
  });
  if (keepBoth) return "keep-both";
  return null;
};

interface ExplorerItemProps {
  entry: ExplorerEntry;
  depth: number;
  onToggleDir: (path: string) => void;
  expanded?: boolean;
}

const getParentPath = (path: string): string | null => {
  const idx = path.lastIndexOf("/");
  return idx > 0 ? path.substring(0, idx) : null;
};

const getFileNameWithoutExt = (name: string): [number, number] => {
  const dotIdx = name.lastIndexOf(".");
  if (dotIdx > 0) return [0, dotIdx];
  return [0, name.length];
};

export const ExplorerItem = ({ entry, depth, onToggleDir, expanded }: ExplorerItemProps) => {
  const selectedPath = useExplorerStore((s) => s.selectedPath);
  const setSelectedPath = useExplorerStore((s) => s.setSelectedPath);
  const setFileContent = useExplorerStore((s) => s.setFileContent);
  const renamingPath = useExplorerStore((s) => s.renamingPath);
  const setRenamingPath = useExplorerStore((s) => s.setRenamingPath);
  const clipboardEntry = useExplorerStore((s) => s.clipboardEntry);
  const setClipboardEntry = useExplorerStore((s) => s.setClipboardEntry);
  const setCreatingIn = useExplorerStore((s) => s.setCreatingIn);
  const dragSource = useExplorerStore((s) => s.dragSource);
  const setDragSource = useExplorerStore((s) => s.setDragSource);
  const dropTarget = useExplorerStore((s) => s.dropTarget);
  const setDropTarget = useExplorerStore((s) => s.setDropTarget);
  const expandDir = useExplorerStore((s) => s.expandDir);
  const clearCache = useExplorerStore((s) => s.clearCache);
  const setError = useRepositoryStore((s) => s.setError);
  const { fileMap, dirMap } = useContext(GitDecorationContext);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const expandTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const decoration = entry.isDirectory ? dirMap.get(entry.path) : fileMap.get(entry.path);

  const isSelected = !entry.isDirectory && selectedPath === entry.path;
  const isRenaming = renamingPath === entry.path;
  const isCut = clipboardEntry?.path === entry.path && clipboardEntry?.operation === "cut";
  const isDragSource = dragSource?.path === entry.path;
  const isDropTarget = entry.isDirectory && dropTarget === entry.path;

  // Auto-focus rename input
  useEffect(() => {
    if (isRenaming && renameInputRef.current) {
      renameInputRef.current.focus();
      const [start, end] = entry.isDirectory ? [0, entry.name.length] : getFileNameWithoutExt(entry.name);
      renameInputRef.current.setSelectionRange(start, end);
    }
  }, [isRenaming, entry.name, entry.isDirectory]);

  // Clean up expand timer on unmount
  useEffect(() => {
    return () => {
      if (expandTimerRef.current) clearTimeout(expandTimerRef.current);
    };
  }, []);

  const handleClick = useCallback(() => {
    if (isRenaming) return;
    if (entry.isDirectory) {
      onToggleDir(entry.path);
      return;
    }
    setSelectedPath(entry.path);
    useLayoutStore.getState().setMainView("file");
    commands.readFileContent(entry.path).then(setFileContent).catch((err) => setError(String(err)));
  }, [entry.path, entry.isDirectory, onToggleDir, setSelectedPath, setFileContent, setError, isRenaming]);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  // --- Rename ---
  const handleRenameSubmit = useCallback(
    async (newName: string) => {
      setRenamingPath(null);
      const trimmed = newName.trim();
      if (!trimmed || trimmed === entry.name) return;
      try {
        await commands.renameEntry(entry.path, trimmed);
        if (selectedPath === entry.path) {
          const parent = getParentPath(entry.path);
          const newPath = parent ? `${parent}/${trimmed}` : trimmed;
          setSelectedPath(newPath);
          commands.readFileContent(newPath).then(setFileContent).catch(() => {});
        }
        clearCache();
      } catch (err) {
        setError(String(err));
      }
    },
    [entry.path, entry.name, selectedPath, setRenamingPath, setSelectedPath, setFileContent, clearCache, setError],
  );

  const handleRenameKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleRenameSubmit(e.currentTarget.value);
      } else if (e.key === "Escape") {
        e.preventDefault();
        setRenamingPath(null);
      }
    },
    [handleRenameSubmit, setRenamingPath],
  );

  // --- Context menu actions ---
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

  const handleDuplicate = useCallback(async () => {
    try {
      await commands.duplicateEntry(entry.path);
      clearCache();
    } catch (err) {
      setError(String(err));
    }
  }, [entry.path, clearCache, setError]);

  const handleDelete = useCallback(async () => {
    const name = entry.name;
    const confirmed = await confirm(`Are you sure you want to delete "${name}"?\n\nThis will move it to the trash.`, {
      title: "Delete",
      kind: "warning",
      okLabel: "Move to Trash",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    try {
      const status = await commands.deleteFile(entry.path);
      useRepositoryStore.getState().setStatus(status);
      if (selectedPath === entry.path) {
        setSelectedPath(null);
        setFileContent(null);
      }
      clearCache();
    } catch (err) {
      setError(String(err));
    }
  }, [entry.path, entry.name, selectedPath, setSelectedPath, setFileContent, clearCache, setError]);

  const handleAddToGitignore = useCallback(async () => {
    try {
      const status = await commands.addToGitignore(entry.path);
      useRepositoryStore.getState().setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [entry.path, setError]);

  const handleCopy = useCallback(() => {
    setClipboardEntry({ path: entry.path, isDirectory: entry.isDirectory, operation: "copy" });
  }, [entry.path, entry.isDirectory, setClipboardEntry]);

  const handleCut = useCallback(() => {
    setClipboardEntry({ path: entry.path, isDirectory: entry.isDirectory, operation: "cut" });
  }, [entry.path, entry.isDirectory, setClipboardEntry]);

  const handlePaste = useCallback(async () => {
    const clip = useExplorerStore.getState().clipboardEntry;
    if (!clip) return;
    const targetDir = entry.isDirectory ? entry.path : (getParentPath(entry.path) ?? "");
    const doPaste = async (resolution?: ConflictResolution) => {
      if (clip.operation === "copy") {
        await commands.copyEntries([clip.path], targetDir, resolution);
      } else {
        await commands.moveEntries([clip.path], targetDir, resolution);
        setClipboardEntry(null);
      }
      clearCache();
    };
    try {
      await doPaste();
    } catch (err) {
      if (isAlreadyExistsError(err)) {
        const resolution = await askConflictResolution();
        if (resolution) {
          try {
            await doPaste(resolution);
          } catch (retryErr) {
            setError(String(retryErr));
          }
        }
      } else {
        setError(String(err));
      }
    }
  }, [entry.path, entry.isDirectory, setClipboardEntry, clearCache, setError]);

  const handleNewFile = useCallback(() => {
    expandDir(entry.path);
    setCreatingIn({ parentPath: entry.path, type: "file" });
  }, [entry.path, expandDir, setCreatingIn]);

  const handleNewFolder = useCallback(() => {
    expandDir(entry.path);
    setCreatingIn({ parentPath: entry.path, type: "folder" });
  }, [entry.path, expandDir, setCreatingIn]);

  // --- Mouse-based drag ---
  const handleMouseDown = useCallback(
    (e: React.MouseEvent) => {
      if (isRenaming || e.button !== 0) return;
      // Store the initial mouse position; drag starts after a small threshold
      const startX = e.clientX;
      const startY = e.clientY;
      let started = false;

      const onMouseMove = (me: MouseEvent) => {
        if (!started) {
          const dx = me.clientX - startX;
          const dy = me.clientY - startY;
          if (Math.abs(dx) + Math.abs(dy) < 5) return;
          started = true;
          setDragSource({ path: entry.path, isDirectory: entry.isDirectory });
        }
        // Find which .explorer-item is under the cursor
        const el = document.elementFromPoint(me.clientX, me.clientY);
        const itemEl = el?.closest(".explorer-item") as HTMLElement | null;
        if (itemEl) {
          const targetPath = itemEl.dataset.path ?? null;
          const targetIsDir = itemEl.dataset.isdir === "true";
          if (targetPath) {
            const dest = targetIsDir ? targetPath : (getParentPath(targetPath) ?? "__root__");
            setDropTarget(dest);
            // Auto-expand collapsed folder
            if (targetIsDir && dest === targetPath) {
              const store = useExplorerStore.getState();
              if (!store.expandedDirs.has(targetPath)) {
                if (expandTimerRef.current) clearTimeout(expandTimerRef.current);
                expandTimerRef.current = setTimeout(() => {
                  onToggleDir(targetPath);
                }, 300);
              }
            }
          }
        } else {
          // Hovering over empty area -> root
          setDropTarget("__root__");
          if (expandTimerRef.current) {
            clearTimeout(expandTimerRef.current);
            expandTimerRef.current = null;
          }
        }
      };

      const cleanup = () => {
        window.removeEventListener("mousemove", onMouseMove);
        window.removeEventListener("mouseup", onMouseUp);
        window.removeEventListener("keydown", onKeyDown);
        if (expandTimerRef.current) {
          clearTimeout(expandTimerRef.current);
          expandTimerRef.current = null;
        }
      };

      const onKeyDown = (ke: KeyboardEvent) => {
        if (ke.key === "Escape") {
          ke.preventDefault();
          cleanup();
          setDragSource(null);
          setDropTarget(null);
        }
      };

      const onMouseUp = async (me: MouseEvent) => {
        cleanup();
        if (!started) return;

        const source = useExplorerStore.getState().dragSource;
        setDragSource(null);
        setDropTarget(null);

        if (!source) return;

        // Determine destination folder, or root ("")
        const el = document.elementFromPoint(me.clientX, me.clientY);
        const itemEl = el?.closest(".explorer-item") as HTMLElement | null;
        let dest: string;
        if (itemEl) {
          const targetPath = itemEl.dataset.path ?? "";
          const targetIsDir = itemEl.dataset.isdir === "true";
          dest = targetIsDir ? targetPath : (getParentPath(targetPath) ?? "");
        } else {
          dest = "";
        }

        // Don't drop onto self or into own parent (no-op)
        const sourceParent = getParentPath(source.path) ?? "";
        if (source.path === dest || sourceParent === dest) return;

        try {
          await commands.moveEntries([source.path], dest);
          clearCache();
        } catch (err) {
          if (isAlreadyExistsError(err)) {
            const resolution = await askConflictResolution();
            if (resolution) {
              try {
                await commands.moveEntries([source.path], dest, resolution);
                clearCache();
              } catch (retryErr) {
                setError(String(retryErr));
              }
            }
          } else {
            setError(String(err));
          }
        }
      };

      window.addEventListener("mousemove", onMouseMove);
      window.addEventListener("mouseup", onMouseUp);
      window.addEventListener("keydown", onKeyDown);
    },
    [entry.path, entry.isDirectory, isRenaming, setDragSource, setDropTarget, onToggleDir, clearCache, setError],
  );

  // --- Build context menu ---
  const getContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [];

    if (entry.isDirectory) {
      items.push(
        { label: "New File...", icon: "new-file", action: handleNewFile },
        { label: "New Folder...", icon: "new-folder", action: handleNewFolder },
        { label: "", action: () => {}, separator: true },
      );
    } else {
      items.push(
        { label: "Open in Editor", icon: "go-to-file", action: handleOpenInEditor },
        { label: "", action: () => {}, separator: true },
      );
    }

    items.push(
      { label: "Rename", icon: "edit", action: () => setRenamingPath(entry.path) },
      { label: "Duplicate", icon: "files", action: handleDuplicate },
      { label: "", action: () => {}, separator: true },
      { label: "Cut", icon: "remove", action: handleCut },
      { label: "Copy", icon: "copy", action: handleCopy },
      {
        label: "Paste",
        icon: "clippy",
        action: handlePaste,
        disabled: !clipboardEntry,
      },
      { label: "", action: () => {}, separator: true },
      { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder },
      { label: "Copy Path", icon: "copy", action: handleCopyPath },
      { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
      { label: "", action: () => {}, separator: true },
      { label: "Move to Trash", icon: "trash", action: handleDelete },
      { label: "Add to .gitignore", icon: "exclude", action: handleAddToGitignore },
    );

    return items;
  };

  const iconClasses = entry.isDirectory
    ? getFileIconClasses(entry.name, "folder")
    : getFileIconClasses(entry.path, "file");

  const classNames = [
    "explorer-item",
    isSelected ? "selected" : "",
    isCut ? "cut" : "",
    isDragSource ? "dragging" : "",
    isDropTarget ? "drop-target" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <>
      <div
        className={classNames}
        style={{ paddingLeft: 12 + depth * 12 }}
        data-path={entry.path}
        data-isdir={entry.isDirectory}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        onMouseDown={handleMouseDown}
      >
        {entry.isDirectory && (
          <span className={`codicon codicon-chevron-down resource-group-chevron${expanded ? "" : " collapsed"}`} />
        )}
        <span className={`resource-item-icon ${iconClasses}`} />
        {isRenaming ? (
          <input
            ref={renameInputRef}
            className="explorer-item-rename-input"
            defaultValue={entry.name}
            onKeyDown={handleRenameKeyDown}
            onBlur={(e) => handleRenameSubmit(e.currentTarget.value)}
            onClick={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            autoComplete="off"
            spellCheck={false}
          />
        ) : (
          <span className="explorer-item-name" style={decoration ? { color: decoration.color } : undefined}>
            {entry.name}
          </span>
        )}
        {decoration && !isRenaming && (
          <span className="explorer-item-status" style={{ color: decoration.color }}>
            {decoration.label}
          </span>
        )}
        {!entry.isDirectory && !isRenaming && (
          <div className="explorer-item-actions">
            <button
              className="inline-action"
              onClick={(e) => {
                e.stopPropagation();
                handleOpenInEditor();
              }}
              onMouseDown={(e) => e.stopPropagation()}
              title="Open in Editor"
            >
              <span className="codicon codicon-go-to-file" />
            </button>
          </div>
        )}
      </div>
      {contextMenu && (
        <ContextMenu x={contextMenu.x} y={contextMenu.y} items={getContextMenuItems()} onClose={() => setContextMenu(null)} />
      )}
    </>
  );
};
