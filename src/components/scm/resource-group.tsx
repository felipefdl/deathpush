import { useState, useCallback, useMemo } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry, ResourceGroup, ResourceGroupKind } from "../../lib/git-types";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import * as commands from "../../lib/tauri-commands";
import { ResourceItem } from "./resource-item";
import { ResourceTree } from "./resource-tree";

interface ResourceGroupHeaderProps {
  collapsed: boolean;
  onToggle: () => void;
  label: string;
  count: number;
  isIndex: boolean;
  onStageAll: () => void;
  onUnstageAll: () => void;
  onDiscardAll: () => void;
}

export const ResourceGroupHeader = ({
  collapsed,
  onToggle,
  label,
  count,
  isIndex,
  onStageAll,
  onUnstageAll,
  onDiscardAll,
}: ResourceGroupHeaderProps) => (
  <div className="resource-group-header" onClick={onToggle}>
    <span className={`codicon codicon-chevron-down resource-group-chevron ${collapsed ? "collapsed" : ""}`} />
    <span className="resource-group-label">{label}</span>
    <span className="resource-group-count">{count}</span>
    <div className="resource-group-actions">
      {isIndex ? (
        <button className="inline-action" onClick={(e) => { e.stopPropagation(); onUnstageAll(); }} title="Unstage All">
          <span className="codicon codicon-remove" />
        </button>
      ) : (
        <>
          <button className="inline-action" onClick={(e) => { e.stopPropagation(); onDiscardAll(); }} title="Discard All Changes">
            <span className="codicon codicon-discard" />
          </button>
          <button className="inline-action" onClick={(e) => { e.stopPropagation(); onStageAll(); }} title="Stage All Changes">
            <span className="codicon codicon-add" />
          </button>
        </>
      )}
    </div>
  </div>
);

interface ResourceGroupBodyProps {
  files: FileEntry[];
  viewMode: "list" | "tree";
  groupKind: ResourceGroupKind;
  flatIndexOffset: number;
  focusedIndex: number | null;
}

export const ResourceGroupBody = ({ files, viewMode, groupKind, flatIndexOffset, focusedIndex }: ResourceGroupBodyProps) => (
  <div className="resource-group-body">
    {viewMode === "tree" ? (
      <ResourceTree files={files} groupKind={groupKind} />
    ) : (
      files.map((file, i) => (
        <ResourceItem
          key={file.path}
          file={file}
          groupKind={groupKind}
          focused={focusedIndex === flatIndexOffset + i}
        />
      ))
    )}
  </div>
);

interface ResourceGroupViewProps {
  group: ResourceGroup;
  filter?: string;
  flatIndexOffset?: number;
}

export const ResourceGroupView = ({ group, filter, flatIndexOffset = 0 }: ResourceGroupViewProps) => {
  const [collapsed, setCollapsed] = useState(false);
  const { setStatus, setError, startOperation, endOperation, focusedIndex } = useRepositoryStore();
  const { viewMode } = useLayoutStore();

  const filteredFiles = useMemo(() => {
    if (!filter) return group.files;
    const lower = filter.toLowerCase();
    return group.files.filter((f) => f.path.toLowerCase().includes(lower));
  }, [group.files, filter]);

  const handleStageAll = useCallback(async () => {
    startOperation("stage");
    try {
      const paths = filteredFiles.map((f) => f.path);
      const status = await commands.stageFiles(paths);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  }, [filteredFiles, setStatus, setError, startOperation, endOperation]);

  const handleUnstageAll = useCallback(async () => {
    startOperation("unstage");
    try {
      const status = await commands.unstageAll();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  }, [setStatus, setError, startOperation, endOperation]);

  const handleDiscardAll = useCallback(async () => {
    const trackedFiles = filteredFiles.filter((f) => f.status !== "untracked");
    const untrackedFiles = filteredFiles.filter((f) => f.status === "untracked");

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
      let status;
      if (trackedFiles.length > 0) {
        status = await commands.discardChanges(trackedFiles.map((f) => f.path));
      }
      if (untrackedFiles.length > 0) {
        status = await commands.deleteFiles(untrackedFiles.map((f) => f.path));
      }
      if (status) setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  }, [filteredFiles, setStatus, setError, startOperation, endOperation]);

  if (filteredFiles.length === 0) return null;

  const isIndex = group.kind === "index";

  return (
    <div className="resource-group">
      <ResourceGroupHeader
        collapsed={collapsed}
        onToggle={() => setCollapsed(!collapsed)}
        label={group.label}
        count={filteredFiles.length}
        isIndex={isIndex}
        onStageAll={handleStageAll}
        onUnstageAll={handleUnstageAll}
        onDiscardAll={handleDiscardAll}
      />
      {!collapsed && (
        <ResourceGroupBody
          files={filteredFiles}
          viewMode={viewMode}
          groupKind={group.kind}
          flatIndexOffset={flatIndexOffset}
          focusedIndex={focusedIndex}
        />
      )}
    </div>
  );
};
