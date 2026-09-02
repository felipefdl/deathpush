import type {
  ContextMenuItem as TreeContextMenuItem,
  FileTree,
  FileTreeDirectoryHandle,
  FileTreeDropResult,
  FileTreeRenameEvent,
} from "@pierre/trees";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { ask } from "@tauri-apps/plugin-dialog";

import { createEffect, createMemo, createSignal, onSettled } from "solid-js";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { shouldRefreshExplorer } from "../../hooks/use-repository-events";
import { addRecentFile } from "../../lib/recent-files";
import { dockTerminalIfCurrentFile, shouldReloadOpenFile } from "../../lib/explorer-file-activate";
import { directoryNeedsChildren, explorerEntriesToTreePaths, explorerGitStatus } from "../../lib/trees";
import { throttle } from "../../lib/throttle";
import type { ExplorerEntry, PathsChanged } from "../../lib/git-types";
import type { ConflictResolution } from "../../lib/tauri-commands";
import * as commands from "../../lib/tauri-commands";
import { sendDestructiveIntent, sendIntent } from "../../lib/session-client";

import { useStore } from "../../lib/use-store";

import { explorerStore } from "../../stores/explorer-store";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import "../../styles/explorer.css";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import { FileTreeHost, type FileTreeHostProps } from "../trees/file-tree-host";
import { renderTreeContextMenu } from "../trees/tree-context-menu";

const isAlreadyExistsError = (error: unknown): boolean => typeof error === "string" && error.includes("already exists");

const stripDirectorySuffix = (path: string): string => (path.endsWith("/") ? path.slice(0, -1) : path);

const getParentPath = (path: string): string => {
  const normalized = stripDirectorySuffix(path);
  const separator = normalized.lastIndexOf("/");
  return separator < 0 ? "" : normalized.slice(0, separator);
};

const getBaseName = (path: string): string => stripDirectorySuffix(path).split("/").pop() ?? path;

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
  return keepBoth ? "keep-both" : null;
};

type ExplorerViewProps = {
  onOpenRepository: () => void;
};

type PendingCreate = {
  path: string;
  type: "file" | "folder";
};

export const ExplorerView = (props: ExplorerViewProps) => {
  const status = useStore(repositoryStore, (state) => state.status);
  const fileFilter = useStore(explorerStore, (state) => state.fileFilter);
  const clipboardEntry = useStore(explorerStore, (state) => state.clipboardEntry);
  const selectedPath = useStore(explorerStore, (state) => state.selectedPath);
  const [entries, setEntries] = createSignal<ExplorerEntry[]>([]);
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number } | null>(null);
  const expandedPaths = useStore(explorerStore, (state) => state.treeExpandedPaths);
  let treeModel: FileTree | undefined;
  let pendingCreate: PendingCreate | undefined;
  const loadedDirectories = new Set<string>();

  const treePaths = createMemo(() => explorerEntriesToTreePaths(entries()));
  const treeGitStatus = createMemo(() =>
    explorerGitStatus(entries(), status()?.groups.flatMap((group) => group.files) ?? [])
  );

  const refreshTree = async (): Promise<void> => {
    loadedDirectories.clear();
    try {
      const next = await commands.listRepositoryTree();
      setEntries(next);
    } catch (error) {
      repositoryStore.getState().setError(String(error));
    }
  };

  const scheduleRefreshTree = throttle(() => {
    void refreshTree();
  }, 1000);

  createEffect(
    () => [entries(), expandedPaths()] as const,
    ([current, expanded]) => {
      for (const path of expanded) {
        const directory = stripDirectorySuffix(path);
        if (!directory || loadedDirectories.has(directory) || !directoryNeedsChildren(current, directory)) continue;
        loadedDirectories.add(directory);
        void commands
          .listRepositoryChildren(directory)
          .then((children) => {
            setEntries((latest) => {
              const byPath = new Map(latest.map((entry) => [entry.path, entry]));
              for (const child of children) byPath.set(child.path, child);
              return [...byPath.values()].sort((left, right) => left.path.localeCompare(right.path));
            });
          })
          .catch((error) => {
            loadedDirectories.delete(directory);
            repositoryStore.getState().setError(String(error));
          });
      }
    }
  );

  const openFile = (path: string): void => {
    const { setSelectedPath, setFileContent } = explorerStore.getState();
    const layout = layoutStore.getState();
    layout.dockTerminal();
    layout.setMainView("file");
    if (!shouldReloadOpenFile(explorerStore.getState().selectedPath, path)) return;
    setSelectedPath(path);
    commands
      .readFileContent(path)
      .then((content) => {
        setFileContent(content);
        const root = repositoryStore.getState().status?.root;
        if (root) addRecentFile(root, path);
      })
      .catch((error) => repositoryStore.getState().setError(String(error)));
  };

  const beginCreate = (parentPath: string | null, type: PendingCreate["type"]): void => {
    if (!treeModel) return;
    const parent = parentPath ? stripDirectorySuffix(parentPath) : "";
    const baseName = type === "folder" ? "New Folder" : "New File";
    let candidate = parent ? `${parent}/${baseName}` : baseName;
    let suffix = 2;
    while (treeModel.getItem(type === "folder" ? `${candidate}/` : candidate)) {
      candidate = `${parent ? `${parent}/` : ""}${baseName} ${suffix}`;
      suffix += 1;
    }
    const treePath = type === "folder" ? `${candidate}/` : candidate;
    pendingCreate = { path: treePath, type };
    treeModel.add(treePath);
    if (parentPath) {
      const parentItem = treeModel.getItem(parentPath);
      if (parentItem?.isDirectory()) (parentItem as FileTreeDirectoryHandle).expand();
    }
    treeModel.startRenaming(treePath, { removeIfCanceled: true });
  };

  const persistRename = async (event: FileTreeRenameEvent): Promise<void> => {
    try {
      if (pendingCreate?.path === event.sourcePath) {
        if (pendingCreate.type === "folder") {
          await commands.createDirectory(stripDirectorySuffix(event.destinationPath));
        } else {
          await commands.writeFile(event.destinationPath, "");
        }
        pendingCreate = undefined;
      } else {
        await commands.renameEntry(stripDirectorySuffix(event.sourcePath), getBaseName(event.destinationPath));
        const explorer = explorerStore.getState();
        if (explorer.selectedPath === stripDirectorySuffix(event.sourcePath)) {
          explorer.setSelectedPath(stripDirectorySuffix(event.destinationPath));
        }
      }
      await refreshTree();
    } catch (error) {
      pendingCreate = undefined;
      repositoryStore.getState().setError(String(error));
      await refreshTree();
    }
  };

  const moveEntries = async (sources: readonly string[], destination: string): Promise<void> => {
    const normalizedSources = sources.map(stripDirectorySuffix);
    const normalizedDestination = stripDirectorySuffix(destination);
    try {
      await commands.moveEntries(normalizedSources, normalizedDestination);
    } catch (error) {
      if (!isAlreadyExistsError(error)) throw error;
      const resolution = await askConflictResolution();
      if (!resolution) return;
      await commands.moveEntries(normalizedSources, normalizedDestination, resolution);
    }
    await refreshTree();
  };

  const persistDrop = (event: FileTreeDropResult): void => {
    void moveEntries(event.draggedPaths, event.target.directoryPath ?? "").catch((error) => {
      repositoryStore.getState().setError(String(error));
      void refreshTree();
    });
  };

  const pasteInto = async (destination: string): Promise<void> => {
    const clip = explorerStore.getState().clipboardEntry;
    if (!clip) return;
    const run = async (resolution?: ConflictResolution): Promise<void> => {
      if (clip.operation === "copy") {
        await commands.copyEntries([clip.path], destination, resolution);
      } else {
        await commands.moveEntries([clip.path], destination, resolution);
        explorerStore.getState().setClipboardEntry(null);
      }
    };
    try {
      await run();
    } catch (error) {
      if (!isAlreadyExistsError(error)) throw error;
      const resolution = await askConflictResolution();
      if (!resolution) return;
      await run(resolution);
    }
    await refreshTree();
  };

  const deleteEntry = async (item: TreeContextMenuItem): Promise<void> => {
    const path = stripDirectorySuffix(item.path);
    const result = await sendDestructiveIntent({ type: "deleteFile", path, confirmed: false });
    if (result.kind !== "snapshot") return;
    const explorer = explorerStore.getState();
    if (explorer.selectedPath === path) {
      explorer.setSelectedPath(null);
      explorer.setFileContent(null);
    }
    await refreshTree();
  };

  const getItemContextMenu = (item: TreeContextMenuItem): ContextMenuItem[] => {
    const path = stripDirectorySuffix(item.path);
    const isDirectory = item.kind === "directory";
    const items: ContextMenuItem[] = [];
    if (isDirectory) {
      items.push(
        { label: "New File...", icon: "new-file", action: () => beginCreate(item.path, "file") },
        { label: "New Folder...", icon: "new-folder", action: () => beginCreate(item.path, "folder") },
        { label: "", action: () => {}, separator: true }
      );
    } else {
      items.push(
        {
          label: "Open in Editor",
          icon: "go-to-file",
          action: () =>
            void commands.openInEditor(path).catch((error) => repositoryStore.getState().setError(String(error))),
        },
        { label: "", action: () => {}, separator: true }
      );
    }
    items.push(
      { label: "Rename", icon: "edit", action: () => treeModel?.startRenaming(item.path) },
      {
        label: "Duplicate",
        icon: "files",
        action: () =>
          void commands
            .duplicateEntry(path)
            .then(refreshTree)
            .catch((error) => repositoryStore.getState().setError(String(error))),
      },
      { label: "", action: () => {}, separator: true },
      {
        label: "Cut",
        icon: "remove",
        action: () => explorerStore.getState().setClipboardEntry({ path, isDirectory, operation: "cut" }),
      },
      {
        label: "Copy",
        icon: "copy",
        action: () => explorerStore.getState().setClipboardEntry({ path, isDirectory, operation: "copy" }),
      },
      {
        label: "Paste",
        icon: "clippy",
        disabled: !clipboardEntry(),
        action: () => void pasteInto(isDirectory ? path : getParentPath(path)),
      },
      { label: "", action: () => {}, separator: true },
      {
        label: "Reveal in Finder",
        icon: "folder-opened",
        action: () =>
          void commands.revealInFileManager(path).catch((error) => repositoryStore.getState().setError(String(error))),
      },
      {
        label: "Copy Path",
        icon: "copy",
        action: () => {
          const root = repositoryStore.getState().status?.root;
          void navigator.clipboard.writeText(root ? `${root}/${path}` : path);
        },
      },
      { label: "Copy Relative Path", icon: "copy", action: () => void navigator.clipboard.writeText(path) },
      { label: "", action: () => {}, separator: true },
      {
        label: "Move to Trash",
        icon: "trash",
        action: () => void deleteEntry(item).catch((error) => repositoryStore.getState().setError(String(error))),
      },
      {
        label: "Add to .gitignore",
        icon: "exclude",
        action: () =>
          void sendIntent({ type: "addToGitignore", path }).catch((error) =>
            repositoryStore.getState().setError(String(error))
          ),
      }
    );
    return items;
  };

  const treeOptions: FileTreeHostProps["options"] = {
    renaming: {
      onRename: (event) => void persistRename(event),
      onError: (error) => repositoryStore.getState().setError(error),
    },
    dragAndDrop: {
      onDropComplete: persistDrop,
      onDropError: (error) => repositoryStore.getState().setError(error),
    },
    onSelectionChange: (selectedPaths) => {
      if (selectedPaths.length !== 1) {
        explorerStore.getState().setSelectedTreeEntry(null);
        return;
      }
      const path = stripDirectorySuffix(selectedPaths[0]);
      const entry = entries().find((candidate) => candidate.path === path);
      explorerStore.getState().setSelectedTreeEntry(entry ? { path, isDirectory: entry.isDirectory } : null);
      if (entry && !entry.isDirectory) openFile(path);
    },
    composition: {
      contextMenu: {
        enabled: true,
        triggerMode: "both",
        render: (item, context) => renderTreeContextMenu(getItemContextMenu(item), context),
      },
    },
  };

  const getRootContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [
      { label: "New File...", icon: "new-file", action: () => beginCreate(null, "file") },
      { label: "New Folder...", icon: "new-folder", action: () => beginCreate(null, "folder") },
    ];
    if (clipboardEntry()) {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Paste", icon: "clippy", action: () => void pasteInto("") }
      );
    }
    return items;
  };

  useTauriEvent<PathsChanged>("repository:paths-changed", (event) => {
    if (!shouldRefreshExplorer(event)) return;
    scheduleRefreshTree();
  });

  createEffect(
    () => fileFilter(),
    (filter) => treeModel?.setSearch(filter || null)
  );

  onSettled(() => {
    const handleRename = (): void => {
      const selected = explorerStore.getState().selectedTreeEntry;
      if (!selected) return;
      treeModel?.startRenaming(selected.isDirectory ? `${selected.path}/` : selected.path);
    };
    window.addEventListener("deathpush:explorer-rename", handleRename);
    return () => window.removeEventListener("deathpush:explorer-rename", handleRename);
  });

  onSettled(() => {
    void refreshTree();
    const unlisten = getCurrentWebviewWindow().onDragDropEvent((event) => {
      if (event.payload.type !== "drop") return;
      commands
        .importFiles(event.payload.paths, "")
        .then(refreshTree)
        .catch((error) => repositoryStore.getState().setError(String(error)));
    });
    return () => void unlisten.then((dispose) => dispose());
  });

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
                onInput={(event) => explorerStore.getState().setFileFilter(event.currentTarget.value)}
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
              <button class="scm-toolbar-button" onClick={() => beginCreate(null, "file")} title="New File">
                <span class="codicon codicon-new-file" />
              </button>
              <button class="scm-toolbar-button" onClick={() => beginCreate(null, "folder")} title="New Folder">
                <span class="codicon codicon-new-folder" />
              </button>
              <button class="scm-toolbar-button" onClick={() => void refreshTree()} title="Refresh Explorer">
                <span class="codicon codicon-refresh" />
              </button>
            </div>
          </div>
          <div
            class="explorer-tree"
            onContextMenu={(event) => {
              if (event.target !== event.currentTarget) return;
              event.preventDefault();
              setContextMenu({ x: event.clientX, y: event.clientY });
            }}
          >
            <FileTreeHost
              paths={treePaths()}
              gitStatus={treeGitStatus()}
              options={treeOptions}
              selectedPath={selectedPath()}
              onFileActivate={(path) => dockTerminalIfCurrentFile(stripDirectorySuffix(path))}
              modelRef={(model) => {
                treeModel = model;
                model?.setSearch(fileFilter() || null);
              }}
            />
          </div>
          {contextMenu() && (
            <ContextMenu
              x={contextMenu()!.x}
              y={contextMenu()!.y}
              items={getRootContextMenuItems()}
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
