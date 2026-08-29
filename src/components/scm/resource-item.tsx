import { createEffect, createSignal } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry, ResourceGroupKind } from "../../lib/git-types";
import { getStatusColor } from "../../lib/status-colors";
import { getStatusLabel } from "../../lib/status-icons";
import { repositoryStore } from "../../stores/repository-store";
import { useDiff } from "../../hooks/use-diff";
import { flushPath, flushPaths } from "../../lib/pierre/flush-registry";
import * as commands from "../../lib/tauri-commands";
import { ContextMenu, type ContextMenuItem } from "./context-menu";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";

type ResourceItemProps = {
  file: FileEntry;
  groupKind: ResourceGroupKind;
  focused?: boolean;
  treeDepth?: number;
};

export const ResourceItem = (props: ResourceItemProps) => {
  const selectedFile = useStore(repositoryStore, (s) => s.selectedFile);
  const selectedFiles = useStore(repositoryStore, (s) => s.selectedFiles);
  const isDiffDirty = useStore(repositoryStore, (s) => s.isDiffDirty);
  const { setStatus, setError, startOperation, endOperation } = repositoryStore.getState();
  const { loadDiff } = useDiff();
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number } | null>(null);
  let itemRef: HTMLDivElement | undefined;

  createEffect(
    () => props.focused,
    (focused) => {
      if (focused && itemRef) {
        itemRef.scrollIntoView({ block: "nearest" });
      }
    }
  );

  const isStaged = () => props.groupKind === "index";
  const selectionKey = () => `${isStaged() ? "staged" : "unstaged"}:${props.file.path}`;
  const isSelected = () => selectedFile()?.path === props.file.path && selectedFile()?.staged === isStaged();
  const isMultiSelected = () => selectedFiles().has(selectionKey());
  const fileName = () => props.file.path.split("/").pop() ?? props.file.path;
  const dirPath = () =>
    props.file.path.includes("/") ? props.file.path.substring(0, props.file.path.lastIndexOf("/")) : "";
  const color = () => getStatusColor(props.file.status);
  const label = () => getStatusLabel(props.file.status);
  const isDeleted = () =>
    props.file.status === "deleted" ||
    props.file.status === "indexDeleted" ||
    props.file.status === "bothDeleted" ||
    props.file.status === "deletedByThem" ||
    props.file.status === "deletedByUs";

  const getSelectedPaths = (prefix: string): string[] => {
    const selected = repositoryStore.getState().selectedFiles;
    const paths: string[] = [];
    for (const key of selected) {
      if (key.startsWith(prefix + ":")) {
        paths.push(key.substring(prefix.length + 1));
      }
    }
    return paths;
  };

  const handleClick = (e: MouseEvent) => {
    const { toggleFileSelection: toggle, clearFileSelection: clear } = repositoryStore.getState();
    if (e.ctrlKey || e.metaKey) {
      toggle(selectionKey(), true, false);
      return;
    }
    if (e.shiftKey) {
      toggle(selectionKey(), false, true);
      return;
    }
    clear();
    void loadDiff(props.file.path, isStaged(), props.groupKind);
    const { mainView, setMainView, dockTerminal } = layoutStore.getState();
    dockTerminal();
    if (mainView !== "changes") setMainView("changes");
  };

  const handleStage = async (e?: MouseEvent) => {
    e?.stopPropagation();
    const state = repositoryStore.getState();
    startOperation("stage");
    try {
      const paths =
        state.selectedFiles.size > 1 && state.selectedFiles.has(selectionKey())
          ? getSelectedPaths("unstaged")
          : [props.file.path];
      if (paths.length === 0) paths.push(props.file.path);
      await flushPaths(paths);
      const status = await commands.stageFiles(paths);
      setStatus(status);
      state.clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstage = async (e?: MouseEvent) => {
    e?.stopPropagation();
    const state = repositoryStore.getState();
    startOperation("unstage");
    try {
      const paths =
        state.selectedFiles.size > 1 && state.selectedFiles.has(selectionKey())
          ? getSelectedPaths("staged")
          : [props.file.path];
      if (paths.length === 0) paths.push(props.file.path);
      await flushPaths(paths);
      const status = await commands.unstageFiles(paths);
      setStatus(status);
      state.clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscard = async (e?: MouseEvent) => {
    e?.stopPropagation();
    const state = repositoryStore.getState();
    const paths =
      state.selectedFiles.size > 1 && state.selectedFiles.has(selectionKey())
        ? getSelectedPaths("unstaged")
        : [props.file.path];
    if (paths.length === 0) paths.push(props.file.path);

    const groups = state.status?.groups ?? [];
    const untrackedSet = new Set<string>();
    for (const g of groups) {
      if (g.kind === "untracked") {
        for (const f of g.files) untrackedSet.add(f.path);
      } else {
        for (const f of g.files) {
          if (f.status === "untracked") untrackedSet.add(f.path);
        }
      }
    }
    const trackedPaths = paths.filter((p) => !untrackedSet.has(p));
    const untrackedPaths = paths.filter((p) => untrackedSet.has(p));

    let msg: string;
    let title: string;
    let okLabel: string;
    if (trackedPaths.length > 0 && untrackedPaths.length > 0) {
      msg = `Are you sure you want to discard changes in ${trackedPaths.length} tracked file(s) and DELETE ${untrackedPaths.length} untracked file(s)?\n\nTracked changes are irreversible. Untracked files can be restored from the Trash.`;
      title = "Discard Changes";
      okLabel = "Discard & Delete";
    } else if (untrackedPaths.length > 0) {
      const names = untrackedPaths.map((p) => p.split("/").pop()).join(", ");
      msg =
        untrackedPaths.length === 1
          ? `Are you sure you want to DELETE the following untracked file: '${names}'?\n\nYou can restore this file from the Trash.`
          : `Are you sure you want to DELETE ${untrackedPaths.length} untracked file(s)?\n\nYou can restore them from the Trash.`;
      title = "Delete Untracked File";
      okLabel = "Move to Trash";
    } else {
      msg =
        paths.length > 1
          ? `Are you sure you want to discard changes in ${paths.length} file(s)?\n\nThis action is irreversible.`
          : `Are you sure you want to discard changes in "${props.file.path.split("/").pop()}"?\n\nThis action is irreversible.`;
      title = "Discard Changes";
      okLabel = "Discard";
    }
    const confirmed = await confirm(msg, { title, kind: "warning", okLabel, cancelLabel: "Cancel" });
    if (!confirmed) return;

    startOperation("discard");
    try {
      await flushPaths(paths);
      let status;
      if (trackedPaths.length > 0) {
        status = await commands.discardChanges(trackedPaths);
      }
      if (untrackedPaths.length > 0) {
        status = await commands.deleteFiles(untrackedPaths);
      }
      if (status) setStatus(status);
      state.clearFileSelection();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  };

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleOpenFile = async () => {
    try {
      await commands.openInEditor(props.file.path);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleRevealInFinder = async () => {
    try {
      await commands.revealInFileManager(props.file.path);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleCopyPath = async () => {
    const root = repositoryStore.getState().status?.root ?? "";
    const fullPath = root ? `${root}/${props.file.path}` : props.file.path;
    await navigator.clipboard.writeText(fullPath);
  };

  const handleCopyRelativePath = async () => {
    await navigator.clipboard.writeText(props.file.path);
  };

  const handleDeleteFile = async () => {
    const confirmed = await confirm(
      `Are you sure you want to move "${props.file.path.split("/").pop()}" to the trash?`,
      { title: "Move to Trash", kind: "warning", okLabel: "Move to Trash", cancelLabel: "Cancel" }
    );
    if (!confirmed) return;
    startOperation("delete");
    try {
      await flushPath(props.file.path);
      const status = await commands.deleteFile(props.file.path);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("delete");
    }
  };

  const handleShowFileHistory = () => {
    layoutStore.getState().setMainView("history");
    window.dispatchEvent(new CustomEvent("deathpush:file-history", { detail: { path: props.file.path } }));
  };

  const handleAddToGitignore = async () => {
    try {
      const status = await commands.addToGitignore(props.file.path);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  };

  const getContextMenuItems = (): ContextMenuItem[] => {
    if (isStaged()) {
      return [
        { label: "Open Changes", icon: "diff", action: () => loadDiff(props.file.path, isStaged(), props.groupKind) },
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
      { label: "Open Changes", icon: "diff", action: () => loadDiff(props.file.path, isStaged(), props.groupKind) },
      { label: "Open File", icon: "go-to-file", action: handleOpenFile },
      { label: "Show File History", icon: "history", action: handleShowFileHistory },
      { label: "", action: () => {}, separator: true },
      { label: "Stage Changes", icon: "add", action: () => handleStage() },
    ];
    if (props.file.status === "untracked") {
      items.push({ label: "Delete", icon: "trash", action: () => handleDiscard() });
    } else {
      items.push({ label: "Discard Changes", icon: "discard", action: () => handleDiscard() });
    }
    items.push(
      { label: "", action: () => {}, separator: true },
      { label: "Copy Path", icon: "copy", action: handleCopyPath },
      { label: "Copy Relative Path", icon: "copy", action: handleCopyRelativePath },
      { label: "Reveal in Finder", icon: "folder-opened", action: handleRevealInFinder }
    );
    if (!isDeleted() && props.file.status !== "untracked") {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Move to Trash", icon: "trash", action: handleDeleteFile }
      );
    }
    if (props.file.status === "untracked") {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Add to .gitignore", icon: "exclude", action: handleAddToGitignore }
      );
    }
    return items;
  };

  const itemClass = () =>
    [
      "resource-item",
      isSelected() ? "selected" : "",
      isMultiSelected() ? "multi-selected" : "",
      props.focused ? "focused" : "",
    ]
      .filter(Boolean)
      .join(" ");

  return (
    <>
      <div
        ref={(el) => {
          itemRef = el;
        }}
        class={itemClass()}
        onClick={handleClick}
        onContextMenu={handleContextMenu}
        style={{
          color: color(),
          ...(props.treeDepth !== undefined ? { "padding-left": `${12 + (props.treeDepth + 1) * 12}px` } : {}),
        }}
      >
        {props.treeDepth !== undefined && <span class="tree-indent-spacer" />}
        <span class={`resource-item-icon ${getFileIconClasses(props.file.path, "file")}`} />
        <span class={`resource-item-name${isDeleted() ? " resource-item-deleted" : ""}`}>
          {fileName()}
          {isSelected() && isDiffDirty() && <span class="dirty-indicator"> *</span>}
        </span>
        {dirPath() && (
          <span class={`resource-item-path${isDeleted() ? " resource-item-deleted" : ""}`}>{dirPath()}</span>
        )}
        <span class="resource-item-spacer" />
        <div class="resource-item-actions">
          {isStaged() ? (
            <button class="inline-action" onClick={(e) => handleUnstage(e)} title="Unstage">
              <span class="codicon codicon-remove" />
            </button>
          ) : (
            <>
              <button
                class="inline-action"
                onClick={(e) => handleDiscard(e)}
                title={props.file.status === "untracked" ? "Delete" : "Discard Changes"}
              >
                <span class={`codicon codicon-${props.file.status === "untracked" ? "trash" : "discard"}`} />
              </button>
              <button class="inline-action" onClick={(e) => handleStage(e)} title="Stage Changes">
                <span class="codicon codicon-add" />
              </button>
            </>
          )}
        </div>
        <span class="resource-item-status" style={{ color: color() }}>
          {label()}
        </span>
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
