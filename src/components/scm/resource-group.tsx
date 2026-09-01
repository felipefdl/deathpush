import { createMemo, createSignal } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry, ResourceGroup, ResourceGroupKind } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { flushPaths } from "../../lib/pierre/flush-registry";
import * as commands from "../../lib/tauri-commands";
import { ResourceTree } from "./resource-tree";

type ResourceGroupHeaderProps = {
  collapsed: boolean;
  onToggle: () => void;
  label: string;
  count: number;
  isIndex: boolean;
  onStageAll: () => void;
  onUnstageAll: () => void;
  onDiscardAll: () => void;
};

export const ResourceGroupHeader = (props: ResourceGroupHeaderProps) => (
  <div class="resource-group-header" onClick={() => props.onToggle()}>
    <span class={`codicon codicon-chevron-down resource-group-chevron ${props.collapsed ? "collapsed" : ""}`} />
    <span class="resource-group-label">{props.label}</span>
    <span class="resource-group-count">{props.count}</span>
    <div class="resource-group-actions">
      {props.isIndex ? (
        <button
          class="inline-action"
          onClick={(e) => {
            e.stopPropagation();
            props.onUnstageAll();
          }}
          title="Unstage All"
        >
          <span class="codicon codicon-remove" />
        </button>
      ) : (
        <>
          <button
            class="inline-action"
            onClick={(e) => {
              e.stopPropagation();
              props.onDiscardAll();
            }}
            title="Discard All Changes"
          >
            <span class="codicon codicon-discard" />
          </button>
          <button
            class="inline-action"
            onClick={(e) => {
              e.stopPropagation();
              props.onStageAll();
            }}
            title="Stage All Changes"
          >
            <span class="codicon codicon-add" />
          </button>
        </>
      )}
    </div>
  </div>
);

type ResourceGroupBodyProps = {
  files: FileEntry[];
  groupKind: ResourceGroupKind;
};

export const ResourceGroupBody = (props: ResourceGroupBodyProps) => (
  <div class="resource-group-body">
    <ResourceTree files={props.files} groupKind={props.groupKind} />
  </div>
);

type ResourceGroupViewProps = {
  group: ResourceGroup;
  filter?: string;
};

export const ResourceGroupView = (props: ResourceGroupViewProps) => {
  const [collapsed, setCollapsed] = createSignal(false);
  const { setError, startOperation, endOperation } = repositoryStore.getState();

  const filteredFiles = createMemo(() => {
    if (!props.filter) return props.group.files;
    const lower = props.filter.toLowerCase();
    return props.group.files.filter((f) => f.path.toLowerCase().includes(lower));
  });

  const handleStageAll = async () => {
    startOperation("stage");
    try {
      const paths = filteredFiles().map((f) => f.path);
      await flushPaths(paths);
      await commands.stageFiles(paths);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstageAll = async () => {
    startOperation("unstage");
    try {
      await commands.unstageAll();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscardAll = async () => {
    const files = filteredFiles();
    const trackedFiles = files.filter((f) => f.status !== "untracked");
    const untrackedFiles = files.filter((f) => f.status === "untracked");

    let msg: string;
    let title: string;
    let okLabel: string;
    if (trackedFiles.length > 0 && untrackedFiles.length > 0) {
      msg = `Are you sure you want to discard ${trackedFiles.length} change(s) and DELETE ${untrackedFiles.length} untracked file(s)?\n\nTracked changes are irreversible. Untracked files can be restored from the Trash.`;
      title = "Discard All Changes";
      okLabel = "Discard & Delete";
    } else if (untrackedFiles.length > 0) {
      msg = `Are you sure you want to DELETE ${untrackedFiles.length} untracked file(s)?\n\nYou can restore them from the Trash.`;
      title = "Delete Untracked Files";
      okLabel = "Move to Trash";
    } else {
      msg = `Are you sure you want to discard all ${trackedFiles.length} change(s)?\n\nThis action is irreversible.`;
      title = "Discard All Changes";
      okLabel = "Discard All";
    }

    const confirmed = await confirm(msg, { title, kind: "warning", okLabel, cancelLabel: "Cancel" });
    if (!confirmed) return;
    startOperation("discard");
    try {
      await flushPaths(files.map((f) => f.path));
      if (trackedFiles.length > 0) {
        await commands.discardChanges(trackedFiles.map((f) => f.path));
      }
      if (untrackedFiles.length > 0) {
        await commands.deleteFiles(untrackedFiles.map((f) => f.path));
      }
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  };

  const isIndex = () => props.group.kind === "index";

  return (
    <>
      {filteredFiles().length === 0 ? null : (
        <div class="resource-group">
          <ResourceGroupHeader
            collapsed={collapsed()}
            onToggle={() => setCollapsed(!collapsed())}
            label={props.group.label}
            count={filteredFiles().length}
            isIndex={isIndex()}
            onStageAll={handleStageAll}
            onUnstageAll={handleUnstageAll}
            onDiscardAll={handleDiscardAll}
          />
          {!collapsed() && <ResourceGroupBody files={filteredFiles()} groupKind={props.group.kind} />}
        </div>
      )}
    </>
  );
};
