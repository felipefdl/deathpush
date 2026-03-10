import { useCallback, useEffect, useState } from "react";
import { ask } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useExplorerGitStatus } from "../../hooks/use-explorer-git-status";
import { useRepositoryStore } from "../../stores/repository-store";
import { useExplorerStore } from "../../stores/explorer-store";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import { ExplorerTree, GitDecorationContext } from "./explorer-tree";
import type { ConflictResolution } from "../../lib/tauri-commands";
import * as commands from "../../lib/tauri-commands";
import "../../styles/explorer.css";

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

interface ExplorerViewProps {
  onOpenRepository: () => void;
}

export const ExplorerView = ({ onOpenRepository }: ExplorerViewProps) => {
  const status = useRepositoryStore((s) => s.status);
  const setError = useRepositoryStore((s) => s.setError);
  const { clearCache, fileFilter, setFileFilter, setCreatingIn, clipboardEntry, setClipboardEntry } = useExplorerStore();
  const dropTarget = useExplorerStore((s) => s.dropTarget);
  const dragSource = useExplorerStore((s) => s.dragSource);
  const gitDecoration = useExplorerGitStatus();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const handleRefresh = useCallback(() => {
    clearCache();
  }, [clearCache]);

  const handleTreeContextMenu = useCallback((e: React.MouseEvent) => {
    if ((e.target as HTMLElement).closest(".explorer-item")) return;
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const getTreeContextMenuItems = useCallback((): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [
      { label: "New File...", icon: "new-file", action: () => setCreatingIn({ parentPath: null, type: "file" }) },
      { label: "New Folder...", icon: "new-folder", action: () => setCreatingIn({ parentPath: null, type: "folder" }) },
    ];
    if (clipboardEntry) {
      items.push(
        { label: "", action: () => {}, separator: true },
        {
          label: "Paste",
          icon: "clippy",
          action: async () => {
            const doPaste = async (resolution?: ConflictResolution) => {
              if (clipboardEntry.operation === "copy") {
                await commands.copyEntries([clipboardEntry.path], "", resolution);
              } else {
                await commands.moveEntries([clipboardEntry.path], "", resolution);
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
          },
        },
      );
    }
    return items;
  }, [clipboardEntry, setClipboardEntry, setCreatingIn, clearCache, setError]);

  // Listen for repository-changed events to refresh expanded dirs
  useTauriEvent(
    "repository-changed",
    useCallback(() => {
      const store = useExplorerStore.getState();
      const rootKey = "__root__";
      commands
        .listDirectory(null)
        .then((result) => {
          store.setDirectoryEntries(rootKey, result);
        })
        .catch(() => {});
      for (const dir of store.expandedDirs) {
        commands
          .listDirectory(dir)
          .then((result) => {
            store.setDirectoryEntries(dir, result);
          })
          .catch(() => {});
      }
    }, []),
  );

  // OS drag-drop: import files from Finder/desktop into repo
  useEffect(() => {
    const unlisten = getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        const targetDir = useExplorerStore.getState().dropTarget ?? "";
        const paths = event.payload.paths;
        commands.importFiles(paths, targetDir).then(() => {
          clearCache();
        }).catch(async (err) => {
          if (isAlreadyExistsError(err)) {
            const resolution = await askConflictResolution();
            if (resolution) {
              try {
                await commands.importFiles(paths, targetDir, resolution);
                clearCache();
              } catch (retryErr) {
                setError(String(retryErr));
              }
            }
          } else {
            setError(String(err));
          }
        });
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [clearCache, setError]);

  if (!status) {
    return (
      <div className="explorer-view">
        <div className="explorer-header">
          <span className="explorer-header-title">Explorer</span>
        </div>
        <div className="scm-empty">
          <span style={{ opacity: 0.5, padding: 16, display: "block", textAlign: "center" }}>No repository open</span>
          <button className="action-button" style={{ margin: "0 16px", width: "auto" }} onClick={onOpenRepository}>
            <span className="codicon codicon-folder-opened" />
            Open Repository
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="explorer-view">
      <div className="explorer-header">
        <div className="explorer-filter">
          <span className="codicon codicon-search explorer-filter-icon" />
          <input
            className="explorer-filter-input"
            type="text"
            placeholder="Filter files..."
            value={fileFilter}
            onChange={(e) => setFileFilter(e.target.value)}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
          />
          {fileFilter && (
            <button className="explorer-filter-clear" onClick={() => setFileFilter("")}>
              <span className="codicon codicon-close" />
            </button>
          )}
        </div>
        <div className="explorer-header-actions">
          <button
            className="scm-toolbar-button"
            onClick={() => setCreatingIn({ parentPath: null, type: "file" })}
            title="New File"
          >
            <span className="codicon codicon-new-file" />
          </button>
          <button
            className="scm-toolbar-button"
            onClick={() => setCreatingIn({ parentPath: null, type: "folder" })}
            title="New Folder"
          >
            <span className="codicon codicon-new-folder" />
          </button>
          <button className="scm-toolbar-button" onClick={handleRefresh} title="Refresh Explorer">
            <span className="codicon codicon-refresh" />
          </button>
        </div>
      </div>
      <div className={`explorer-tree${dragSource && dropTarget === "__root__" ? " root-drop-target" : ""}`} onContextMenu={handleTreeContextMenu}>
        <GitDecorationContext.Provider value={gitDecoration}>
          <ExplorerTree />
        </GitDecorationContext.Provider>
      </div>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getTreeContextMenuItems()}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
