import type { ContextMenuItem as TreeContextMenuItem, FileTree } from "@pierre/trees";
import type { FileEntry, ResourceGroupKind } from "../../lib/git-types";
import { fileEntriesToTreeGitStatus } from "../../lib/trees";
import { flushPath, flushPaths } from "../../lib/pierre/flush-registry";
import { sendDestructiveIntent, sendIntent } from "../../lib/session-client";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { useDiff } from "../../hooks/use-diff";
import * as commands from "../../lib/tauri-commands";
import { FileTreeHost, type FileTreeHostProps } from "../trees/file-tree-host";
import { renderTreeContextMenu } from "../trees/tree-context-menu";
import type { ContextMenuItem } from "./context-menu";

type ResourceTreeProps = {
  files: FileEntry[];
  groupKind: ResourceGroupKind;
};

export const ResourceTree = (props: ResourceTreeProps) => {
  const { loadDiff } = useDiff();
  const isStaged = props.groupKind === "index";
  const isUntracked = props.groupKind === "untracked";
  let treeModel: FileTree | undefined;

  const setError = (error: unknown): void => repositoryStore.getState().setError(String(error));

  const finishOperation = (operation: string): void => repositoryStore.getState().endOperation(operation);

  const selectedPathsForItem = (item: TreeContextMenuItem): string[] => {
    const selected = treeModel?.getSelectedPaths() ?? [];
    return selected.includes(item.path) ? selected : [item.path];
  };


  const showDiff = (file: FileEntry): void => {
    void loadDiff(file.path, isStaged, props.groupKind);
    const layout = layoutStore.getState();
    layout.dockTerminal();
    if (layout.mainView !== "changes") layout.setMainView("changes");
  };

  const stage = async (paths: string[]): Promise<void> => {
    const store = repositoryStore.getState();
    store.startOperation("stage");
    try {
      await flushPaths(paths);
      await sendIntent({ type: "stage", paths });
    } catch (error) {
      setError(error);
    } finally {
      finishOperation("stage");
    }
  };

  const unstage = async (paths: string[]): Promise<void> => {
    const store = repositoryStore.getState();
    store.startOperation("unstage");
    try {
      await flushPaths(paths);
      await sendIntent({ type: "unstage", paths });
    } catch (error) {
      setError(error);
    } finally {
      finishOperation("unstage");
    }
  };

  const discard = async (paths: string[]): Promise<void> => {
    const store = repositoryStore.getState();
    store.startOperation("discard");
    try {
      await flushPaths(paths);
      await sendDestructiveIntent({ type: "discard", paths, confirmed: false });
    } catch (error) {
      setError(error);
    } finally {
      finishOperation("discard");
    }
  };

  const deleteFile = async (file: FileEntry): Promise<void> => {
    const store = repositoryStore.getState();
    store.startOperation("delete");
    try {
      await flushPath(file.path);
      await sendDestructiveIntent({ type: "deleteFile", path: file.path, confirmed: false });
    } catch (error) {
      setError(error);
    } finally {
      finishOperation("delete");
    }
  };

  const getContextMenuItems = (item: TreeContextMenuItem): ContextMenuItem[] => {
    const paths = selectedPathsForItem(item);
    const file = props.files.find((candidate) => candidate.path === item.path);
    const items: ContextMenuItem[] = [];
    if (file) {
      items.push(
        { label: "Open Changes", icon: "diff", action: () => showDiff(file) },
        {
          label: "Open File",
          icon: "go-to-file",
          action: () => void commands.openInEditor(file.path).catch(setError),
        },
        {
          label: "Show File History",
          icon: "history",
          action: () => {
            layoutStore.getState().setMainView("history");
            window.dispatchEvent(new CustomEvent("deathpush:file-history", { detail: { path: file.path } }));
          },
        },
        { label: "", action: () => {}, separator: true }
      );
    }
    items.push(
      isStaged
        ? { label: "Unstage Changes", icon: "remove", action: () => void unstage(paths) }
        : { label: "Stage Changes", icon: "add", action: () => void stage(paths) },
      { label: "", action: () => {}, separator: true }
    );
    if (!isStaged) {
      items.push({
        label: isUntracked ? "Delete" : "Discard Changes",
        icon: isUntracked ? "trash" : "discard",
        action: () => void discard(paths),
      });
    }
    if (file) {
      items.push(
        { label: "", action: () => {}, separator: true },
        {
          label: "Copy Path",
          icon: "copy",
          action: () => {
            const root = repositoryStore.getState().status?.root;
            void navigator.clipboard.writeText(root ? `${root}/${file.path}` : file.path);
          },
        },
        { label: "Copy Relative Path", icon: "copy", action: () => void navigator.clipboard.writeText(file.path) },
        {
          label: "Reveal in Finder",
          icon: "folder-opened",
          action: () => void commands.revealInFileManager(file.path).catch(setError),
        }
      );
      if (!isStaged && !isUntracked) {
        items.push(
          { label: "", action: () => {}, separator: true },
          { label: "Move to Trash", icon: "trash", action: () => void deleteFile(file) }
        );
      }
      if (isUntracked) {
        items.push(
          { label: "", action: () => {}, separator: true },
          {
            label: "Add to .gitignore",
            icon: "exclude",
            action: () => void sendIntent({ type: "addToGitignore", path: file.path }).catch(setError),
          }
        );
      }
    }
    return items;
  };


  const treeOptions: FileTreeHostProps["options"] = {
    onSelectionChange: (selectedPaths) => {
      if (selectedPaths.length !== 1) return;
      const file = props.files.find((candidate) => candidate.path === selectedPaths[0]);
      if (file) showDiff(file);
    },
    composition: {
      contextMenu: {
        enabled: true,
        triggerMode: "both",
        render: (item, context) => renderTreeContextMenu(getContextMenuItems(item), context),
      },
    },
  };

  return (
    <FileTreeHost
      paths={props.files.map((file) => file.path)}
      gitStatus={fileEntriesToTreeGitStatus(props.files)}
      options={treeOptions}
      onFileActivate={(path) => {
        const file = props.files.find((candidate) => candidate.path === path);
        if (file) showDiff(file);
      }}
      modelRef={(model) => {
        treeModel = model;
      }}
      class="resource-tree"
    />
  );
};
