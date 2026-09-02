import { createMemo, createSignal } from "solid-js";
import type { FileEntry, ResourceGroup, ResourceGroupKind } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { flushPaths } from "../../lib/pierre/flush-registry";
import { sendDestructiveIntent, sendIntent } from "../../lib/session-client";

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
      await sendIntent({ type: "stage", paths });
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstageAll = async () => {
    startOperation("unstage");
    try {
      await sendIntent({ type: "unstageAll" });
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscardAll = async () => {
    const files = filteredFiles();
    startOperation("discard");
    try {
      await flushPaths(files.map((f) => f.path));
      await sendDestructiveIntent({
        type: "discard",
        paths: files.map((f) => f.path),
        confirmed: false,
      });
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
