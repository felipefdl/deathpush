import { createEffect, createMemo, createSignal, onSettled, useContext } from "solid-js";
import { ask, confirm } from "@tauri-apps/plugin-dialog";
import type { ExplorerEntry } from "../../lib/git-types";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import type { ConflictResolution } from "../../lib/tauri-commands";
import { explorerStore } from "../../stores/explorer-store";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import { GitDecorationContext } from "./explorer-tree";
import { addRecentFile } from "../../lib/recent-files";
import * as commands from "../../lib/tauri-commands";

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

type ExplorerItemProps = {
  entry: ExplorerEntry;
  depth: number;
  onToggleDir: (path: string) => void;
  expanded?: boolean;
};

const getParentPath = (path: string): string | null => {
  const idx = path.lastIndexOf("/");
  return idx > 0 ? path.substring(0, idx) : null;
};

const getFileNameWithoutExt = (name: string): [number, number] => {
  const dotIdx = name.lastIndexOf(".");
  if (dotIdx > 0) return [0, dotIdx];
  return [0, name.length];
};

export const ExplorerItem = (props: ExplorerItemProps) => {
  const selectedPath = useStore(explorerStore, (s) => s.selectedPath);
  const renamingPath = useStore(explorerStore, (s) => s.renamingPath);
  const clipboardEntry = useStore(explorerStore, (s) => s.clipboardEntry);
  const dragSource = useStore(explorerStore, (s) => s.dragSource);
  const dropTarget = useStore(explorerStore, (s) => s.dropTarget);
  const gitDecoration = useContext(GitDecorationContext);
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number } | null>(null);
  let renameInputRef: HTMLInputElement | undefined;
  let expandTimer: ReturnType<typeof setTimeout> | undefined;

  const decoration = createMemo(() => {
    const maps = gitDecoration();
    return props.entry.isDirectory ? maps.dirMap.get(props.entry.path) : maps.fileMap.get(props.entry.path);
  });

  const isSelected = createMemo(() => !props.entry.isDirectory && selectedPath() === props.entry.path);
  const isRenaming = createMemo(() => renamingPath() === props.entry.path);
  const isCut = createMemo(() => clipboardEntry()?.path === props.entry.path && clipboardEntry()?.operation === "cut");
  const isDragSource = createMemo(() => dragSource()?.path === props.entry.path);
  const isDropTarget = createMemo(() => props.entry.isDirectory && dropTarget() === props.entry.path);

  createEffect(
    () => [isRenaming(), props.entry.name, props.entry.isDirectory] as const,
    ([renaming, name, isDirectory]) => {
      if (renaming && renameInputRef) {
        renameInputRef.focus();
        const [start, end] = isDirectory ? [0, name.length] : getFileNameWithoutExt(name);
        renameInputRef.setSelectionRange(start, end);
      }
    }
  );

  onSettled(() => {
    return () => {
      if (expandTimer) clearTimeout(expandTimer);
    };
  });

  const handleClick = () => {
    if (isRenaming()) return;
    if (props.entry.isDirectory) {
      props.onToggleDir(props.entry.path);
      return;
    }
    const { setSelectedPath, setFileContent } = explorerStore.getState();
    setSelectedPath(props.entry.path);
    const layout = layoutStore.getState();
    layout.dockTerminal();
    layout.setMainView("file");
    commands
      .readFileContent(props.entry.path)
      .then((content) => {
        setFileContent(content);
        const root = repositoryStore.getState().status?.root;
        if (root) addRecentFile(root, props.entry.path);
      })
      .catch((err) => repositoryStore.getState().setError(String(err)));
  };

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleRenameSubmit = async (newName: string) => {
    const {
      setRenamingPath,
      setSelectedPath,
      setFileContent,
      clearCache,
      selectedPath: current,
    } = explorerStore.getState();
    setRenamingPath(null);
    const trimmed = newName.trim();
    if (!trimmed || trimmed === props.entry.name) return;
    try {
      await commands.renameEntry(props.entry.path, trimmed);
      if (current === props.entry.path) {
        const parent = getParentPath(props.entry.path);
        const newPath = parent ? `${parent}/${trimmed}` : trimmed;
        setSelectedPath(newPath);
        commands
          .readFileContent(newPath)
          .then(setFileContent)
          .catch(() => {});
      }
      clearCache();
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleRenameKeyDown = (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleRenameSubmit(e.currentTarget.value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      explorerStore.getState().setRenamingPath(null);
    }
  };

  const handleOpenInEditor = async () => {
    try {
      await commands.openInEditor(props.entry.path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleRevealInFinder = async () => {
    try {
      await commands.revealInFileManager(props.entry.path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleCopyPath = async () => {
    const root = repositoryStore.getState().status?.root ?? "";
    const fullPath = root ? `${root}/${props.entry.path}` : props.entry.path;
    await navigator.clipboard.writeText(fullPath);
  };

  const handleCopyRelativePath = async () => {
    await navigator.clipboard.writeText(props.entry.path);
  };

  const handleDuplicate = async () => {
    try {
      await commands.duplicateEntry(props.entry.path);
      explorerStore.getState().clearCache();
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleDelete = async () => {
    const name = props.entry.name;
    const confirmed = await confirm(`Are you sure you want to delete "${name}"?\n\nThis will move it to the trash.`, {
      title: "Delete",
      kind: "warning",
      okLabel: "Move to Trash",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    try {
      const status = await commands.deleteFile(props.entry.path);
      repositoryStore.getState().setStatus(status);
      const { selectedPath: current, setSelectedPath, setFileContent, clearCache } = explorerStore.getState();
      if (current === props.entry.path) {
        setSelectedPath(null);
        setFileContent(null);
      }
      clearCache();
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleAddToGitignore = async () => {
    try {
      const status = await commands.addToGitignore(props.entry.path);
      repositoryStore.getState().setStatus(status);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleCopy = () => {
    explorerStore.getState().setClipboardEntry({
      path: props.entry.path,
      isDirectory: props.entry.isDirectory,
      operation: "copy",
    });
  };

  const handleCut = () => {
    explorerStore.getState().setClipboardEntry({
      path: props.entry.path,
      isDirectory: props.entry.isDirectory,
      operation: "cut",
    });
  };

  const handlePaste = async () => {
    const clip = explorerStore.getState().clipboardEntry;
    if (!clip) return;
    const { setClipboardEntry, clearCache } = explorerStore.getState();
    const targetDir = props.entry.isDirectory ? props.entry.path : (getParentPath(props.entry.path) ?? "");
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
            repositoryStore.getState().setError(String(retryErr));
          }
        }
      } else {
        repositoryStore.getState().setError(String(err));
      }
    }
  };

  const handleNewFile = () => {
    explorerStore.getState().expandDir(props.entry.path);
    explorerStore.getState().setCreatingIn({ parentPath: props.entry.path, type: "file" });
  };

  const handleNewFolder = () => {
    explorerStore.getState().expandDir(props.entry.path);
    explorerStore.getState().setCreatingIn({ parentPath: props.entry.path, type: "folder" });
  };

  const handleMouseDown = (e: MouseEvent) => {
    if (isRenaming() || e.button !== 0) return;
    const startX = e.clientX;
    const startY = e.clientY;
    let started = false;

    const onMouseMove = (me: MouseEvent) => {
      if (!started) {
        const dx = me.clientX - startX;
        const dy = me.clientY - startY;
        if (Math.abs(dx) + Math.abs(dy) < 5) return;
        started = true;
        explorerStore.getState().setDragSource({ path: props.entry.path, isDirectory: props.entry.isDirectory });
      }
      const el = document.elementFromPoint(me.clientX, me.clientY);
      const itemEl = el?.closest(".explorer-item") as HTMLElement | null;
      if (itemEl) {
        const targetPath = itemEl.dataset.path ?? null;
        const targetIsDir = itemEl.dataset.isdir === "true";
        if (targetPath) {
          const dest = targetIsDir ? targetPath : (getParentPath(targetPath) ?? "__root__");
          explorerStore.getState().setDropTarget(dest);
          if (targetIsDir && dest === targetPath) {
            const store = explorerStore.getState();
            if (!store.expandedDirs.has(targetPath)) {
              if (expandTimer) clearTimeout(expandTimer);
              expandTimer = setTimeout(() => {
                props.onToggleDir(targetPath);
              }, 300);
            }
          }
        }
      } else {
        explorerStore.getState().setDropTarget("__root__");
        if (expandTimer) {
          clearTimeout(expandTimer);
          expandTimer = undefined;
        }
      }
    };

    const cleanup = () => {
      window.removeEventListener("mousemove", onMouseMove);
      window.removeEventListener("mouseup", onMouseUp);
      window.removeEventListener("keydown", onKeyDown);
      if (expandTimer) {
        clearTimeout(expandTimer);
        expandTimer = undefined;
      }
    };

    const onKeyDown = (ke: KeyboardEvent) => {
      if (ke.key === "Escape") {
        ke.preventDefault();
        cleanup();
        explorerStore.getState().setDragSource(null);
        explorerStore.getState().setDropTarget(null);
      }
    };

    const onMouseUp = async (me: MouseEvent) => {
      cleanup();
      if (!started) return;

      const source = explorerStore.getState().dragSource;
      explorerStore.getState().setDragSource(null);
      explorerStore.getState().setDropTarget(null);

      if (!source) return;

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

      const sourceParent = getParentPath(source.path) ?? "";
      if (source.path === dest || sourceParent === dest) return;

      try {
        await commands.moveEntries([source.path], dest);
        explorerStore.getState().clearCache();
      } catch (err) {
        if (isAlreadyExistsError(err)) {
          const resolution = await askConflictResolution();
          if (resolution) {
            try {
              await commands.moveEntries([source.path], dest, resolution);
              explorerStore.getState().clearCache();
            } catch (retryErr) {
              repositoryStore.getState().setError(String(retryErr));
            }
          }
        } else {
          repositoryStore.getState().setError(String(err));
        }
      }
    };

    window.addEventListener("mousemove", onMouseMove);
    window.addEventListener("mouseup", onMouseUp);
    window.addEventListener("keydown", onKeyDown);
  };

  const getContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [];

    if (props.entry.isDirectory) {
      items.push(
        { label: "New File...", icon: "new-file", action: handleNewFile },
        { label: "New Folder...", icon: "new-folder", action: handleNewFolder },
        { label: "", action: () => {}, separator: true }
      );
    } else {
      items.push(
        { label: "Open in Editor", icon: "go-to-file", action: handleOpenInEditor },
        { label: "", action: () => {}, separator: true }
      );
    }

    items.push(
      { label: "Rename", icon: "edit", action: () => explorerStore.getState().setRenamingPath(props.entry.path) },
      { label: "Duplicate", icon: "files", action: handleDuplicate },
      { label: "", action: () => {}, separator: true },
      { label: "Cut", icon: "remove", action: handleCut },
      { label: "Copy", icon: "copy", action: handleCopy },
      {
        label: "Paste",
        icon: "clippy",
        action: handlePaste,
        disabled: !clipboardEntry(),
      },
      { label: "", action: () => {}, separator: true },
      { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder },
      { label: "Copy Path", icon: "copy", action: handleCopyPath },
      { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
      { label: "", action: () => {}, separator: true },
      { label: "Move to Trash", icon: "trash", action: handleDelete },
      { label: "Add to .gitignore", icon: "exclude", action: handleAddToGitignore }
    );

    return items;
  };

  const iconClasses = createMemo(() =>
    props.entry.isDirectory
      ? getFileIconClasses(props.entry.name, "folder")
      : getFileIconClasses(props.entry.path, "file")
  );

  const classNames = createMemo(() =>
    [
      "explorer-item",
      isSelected() ? "selected" : "",
      isCut() ? "cut" : "",
      isDragSource() ? "dragging" : "",
      isDropTarget() ? "drop-target" : "",
    ]
      .filter(Boolean)
      .join(" ")
  );

  return (
    <>
      <div
        class={classNames()}
        style={{ "padding-left": `${12 + props.depth * 12}px` }}
        data-path={props.entry.path}
        data-isdir={props.entry.isDirectory ? "true" : "false"}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        onMouseDown={handleMouseDown}
      >
        {props.entry.isDirectory ? (
          <span class={`codicon codicon-chevron-down resource-group-chevron${props.expanded ? "" : " collapsed"}`} />
        ) : (
          <span class="tree-indent-spacer" />
        )}
        <span class={`resource-item-icon ${iconClasses()}`} />
        {isRenaming() ? (
          <input
            ref={(el) => {
              renameInputRef = el;
              if (el) el.value = props.entry.name;
            }}
            class="explorer-item-rename-input"
            onKeyDown={handleRenameKeyDown}
            onBlur={(e) => handleRenameSubmit(e.currentTarget.value)}
            onClick={(e) => e.stopPropagation()}
            onMouseDown={(e) => e.stopPropagation()}
            autocomplete="off"
            spellcheck={false}
          />
        ) : (
          <span class="explorer-item-name" style={decoration() ? { color: decoration()!.color } : undefined}>
            {props.entry.name}
          </span>
        )}
        {decoration() && !isRenaming() && (
          <span class="explorer-item-status" style={{ color: decoration()!.color }}>
            {decoration()!.label}
          </span>
        )}
        {!props.entry.isDirectory && !isRenaming() && (
          <div class="explorer-item-actions">
            <button
              class="inline-action"
              onClick={(e) => {
                e.stopPropagation();
                void handleOpenInEditor();
              }}
              onMouseDown={(e) => e.stopPropagation()}
              title="Open in Editor"
            >
              <span class="codicon codicon-go-to-file" />
            </button>
          </div>
        )}
      </div>
      {contextMenu() && (
        <ContextMenu
          x={contextMenu()!.x}
          y={contextMenu()!.y}
          items={getContextMenuItems()}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
};
