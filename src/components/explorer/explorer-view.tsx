import { createSignal, onSettled } from "solid-js";
import { ask } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useExplorerGitStatus } from "../../hooks/use-explorer-git-status";
import { repositoryStore } from "../../stores/repository-store";
import { explorerStore } from "../../stores/explorer-store";
import { useStore } from "../../lib/use-store";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import { ExplorerTree, GitDecorationContext } from "./explorer-tree";
import type { ConflictResolution } from "../../lib/tauri-commands";
import * as commands from "../../lib/tauri-commands";
import "../../styles/explorer.css";

const isAlreadyExistsError = (err: unknown): boolean => typeof err === "string" && err.includes("already exists");

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

type ExplorerViewProps = {
  onOpenRepository: () => void;
};

export const ExplorerView = (props: ExplorerViewProps) => {
  const status = useStore(repositoryStore, (s) => s.status);
  const fileFilter = useStore(explorerStore, (s) => s.fileFilter);
  const clipboardEntry = useStore(explorerStore, (s) => s.clipboardEntry);
  const dropTarget = useStore(explorerStore, (s) => s.dropTarget);
  const dragSource = useStore(explorerStore, (s) => s.dragSource);
  const gitDecoration = useExplorerGitStatus();
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number } | null>(null);

  const handleRefresh = () => {
    explorerStore.getState().clearCache();
  };

  const handleTreeContextMenu = (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest(".explorer-item")) return;
    e.preventDefault();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const getTreeContextMenuItems = (): ContextMenuItem[] => {
    const clip = clipboardEntry();
    const { setCreatingIn, setClipboardEntry, clearCache } = explorerStore.getState();
    const items: ContextMenuItem[] = [
      { label: "New File...", icon: "new-file", action: () => setCreatingIn({ parentPath: null, type: "file" }) },
      { label: "New Folder...", icon: "new-folder", action: () => setCreatingIn({ parentPath: null, type: "folder" }) },
    ];
    if (clip) {
      items.push(
        { label: "", action: () => {}, separator: true },
        {
          label: "Paste",
          icon: "clippy",
          action: async () => {
            const doPaste = async (resolution?: ConflictResolution) => {
              if (clip.operation === "copy") {
                await commands.copyEntries([clip.path], "", resolution);
              } else {
                await commands.moveEntries([clip.path], "", resolution);
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
                    repositoryStore.getState().setError(String(retryErr));
                  }
                }
              } else {
                repositoryStore.getState().setError(String(err));
              }
            }
          },
        }
      );
    }
    return items;
  };

  useTauriEvent("repository-changed", () => {
    const store = explorerStore.getState();
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
  });

  onSettled(() => {
    const unlisten = getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type === "drop") {
        const { dropTarget: target, clearCache } = explorerStore.getState();
        const targetDir = target ?? "";
        const paths = event.payload.paths;
        commands
          .importFiles(paths, targetDir)
          .then(() => {
            clearCache();
          })
          .catch(async (err) => {
            if (isAlreadyExistsError(err)) {
              const resolution = await askConflictResolution();
              if (resolution) {
                try {
                  await commands.importFiles(paths, targetDir, resolution);
                  clearCache();
                } catch (retryErr) {
                  repositoryStore.getState().setError(String(retryErr));
                }
              }
            } else {
              repositoryStore.getState().setError(String(err));
            }
          });
      }
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  });

  const handleFilterInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    explorerStore.getState().setFileFilter(e.currentTarget.value);
  };

  const treeClass = () => `explorer-tree${dragSource() && dropTarget() === "__root__" ? " root-drop-target" : ""}`;

  return (
    <div class="explorer-view">
      {status() ? (
        <>
          <div class="explorer-header">
            <div class="explorer-filter">
              <span class="codicon codicon-search explorer-filter-icon" />
              <input
                class="explorer-filter-input"
                type="text"
                placeholder="Filter files..."
                value={fileFilter()}
                onInput={handleFilterInput}
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck={false}
              />
              {fileFilter() && (
                <button class="explorer-filter-clear" onClick={() => explorerStore.getState().setFileFilter("")}>
                  <span class="codicon codicon-close" />
                </button>
              )}
            </div>
            <div class="explorer-header-actions">
              <button
                class="scm-toolbar-button"
                onClick={() => explorerStore.getState().setCreatingIn({ parentPath: null, type: "file" })}
                title="New File"
              >
                <span class="codicon codicon-new-file" />
              </button>
              <button
                class="scm-toolbar-button"
                onClick={() => explorerStore.getState().setCreatingIn({ parentPath: null, type: "folder" })}
                title="New Folder"
              >
                <span class="codicon codicon-new-folder" />
              </button>
              <button class="scm-toolbar-button" onClick={handleRefresh} title="Refresh Explorer">
                <span class="codicon codicon-refresh" />
              </button>
            </div>
          </div>
          <div class={treeClass()} onContextMenu={handleTreeContextMenu}>
            <GitDecorationContext value={gitDecoration}>
              <ExplorerTree />
            </GitDecorationContext>
          </div>
          {contextMenu() && (
            <ContextMenu
              x={contextMenu()!.x}
              y={contextMenu()!.y}
              items={getTreeContextMenuItems()}
              onClose={() => setContextMenu(null)}
            />
          )}
        </>
      ) : (
        <>
          <div class="explorer-header">
            <span class="explorer-header-title">Explorer</span>
          </div>
          <div class="scm-empty">
            <span style={{ opacity: 0.5, padding: "16px", display: "block", "text-align": "center" }}>
              No repository open
            </span>
            <button class="action-button" style={{ margin: "0 16px", width: "auto" }} onClick={props.onOpenRepository}>
              <span class="codicon codicon-folder-opened" />
              Open Repository
            </button>
          </div>
        </>
      )}
    </div>
  );
};
