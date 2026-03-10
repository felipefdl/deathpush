import { useMemo, useCallback, useEffect } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry } from "../../lib/git-types";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useColorScheme } from "../../hooks/use-color-scheme";
import { useGitStatus } from "../../hooks/use-git-status";
import { useStash } from "../../hooks/use-stash";
import { CommitInput } from "./commit-input";
import { FileFilter } from "./file-filter";
import { MergeBanner } from "./merge-banner";
import { ResourceGroupHeader, ResourceGroupBody } from "./resource-group";
import { StashHeader, StashBody } from "./stash-view";
import { SubReposHeader, SubReposBody, useSubRepos } from "./sub-repos-view";
import { ScmToolbar } from "./scm-toolbar";
import { ResizablePaneContainer, type PaneDefinition } from "./resizable-pane-container";
import * as commands from "../../lib/tauri-commands";
import "../../styles/scm.css";
import "../../styles/repositories.css";

const GROUP_RATIOS: Record<string, number> = {
  merge: 1,
  index: 0.8,
  workingTree: 1,
  untracked: 0.4,
};

interface ScmViewProps {
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
}

export const ScmView = ({ onOpenRepository, onCloneRepository }: ScmViewProps) => {
  const {
    status,
    stashes,
    fileFilter,
    setStatus,
    setError,
    startOperation,
    endOperation,
    focusedIndex,
  } = useRepositoryStore();
  const { viewMode } = useLayoutStore();
  const colorScheme = useColorScheme();
  const isDark = colorScheme === "dark";
  useGitStatus();
  const { loadStashes, applyStash, popStash, dropStash } = useStash();
  const { repos: subRepos } = useSubRepos();

  useEffect(() => {
    if (status) {
      loadStashes();
    }
  }, [status, loadStashes]);

  const filteredGroups = useMemo(() => {
    if (!status) return [];
    const lower = fileFilter.toLowerCase();
    return status.groups.map((group) => {
      const files = fileFilter
        ? group.files.filter((f) => f.path.toLowerCase().includes(lower))
        : group.files;
      return { group, files };
    }).filter(({ files }) => files.length > 0);
  }, [status, fileFilter]);

  const groupOffsets = useMemo(() => {
    const offsets: number[] = [];
    let offset = 0;
    for (const { files } of filteredGroups) {
      offsets.push(offset);
      offset += files.length;
    }
    return offsets;
  }, [filteredGroups]);

  const handleStageAll = useCallback(async (paths: string[]) => {
    startOperation("stage");
    try {
      const s = await commands.stageFiles(paths);
      setStatus(s);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  }, [setStatus, setError, startOperation, endOperation]);

  const handleUnstageAll = useCallback(async () => {
    startOperation("unstage");
    try {
      const s = await commands.unstageAll();
      setStatus(s);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  }, [setStatus, setError, startOperation, endOperation]);

  const handleDiscardAll = useCallback(async (files: FileEntry[]) => {
    const trackedPaths = files.filter((f) => f.status !== "untracked").map((f) => f.path);
    const untrackedPaths = files.filter((f) => f.status === "untracked").map((f) => f.path);

    let msg: string;
    let title: string;
    let okLabel: string;
    if (trackedPaths.length > 0 && untrackedPaths.length > 0) {
      msg = `Are you sure you want to discard ${trackedPaths.length} change(s) and DELETE ${untrackedPaths.length} untracked file(s)?\n\nTracked changes are irreversible. Untracked files can be restored from the Trash.`;
      title = "Discard All Changes";
      okLabel = "Discard & Delete";
    } else if (untrackedPaths.length > 0) {
      msg = `Are you sure you want to DELETE ${untrackedPaths.length} untracked file(s)?\n\nYou can restore them from the Trash.`;
      title = "Delete Untracked Files";
      okLabel = "Move to Trash";
    } else {
      msg = `Are you sure you want to discard all ${trackedPaths.length} change(s)?\n\nThis action is irreversible.`;
      title = "Discard All Changes";
      okLabel = "Discard All";
    }

    const confirmed = await confirm(msg, { title, kind: "warning", okLabel, cancelLabel: "Cancel" });
    if (!confirmed) return;
    startOperation("discard");
    try {
      let s;
      if (trackedPaths.length > 0) {
        s = await commands.discardChanges(trackedPaths);
      }
      if (untrackedPaths.length > 0) {
        s = await commands.deleteFiles(untrackedPaths);
      }
      if (s) setStatus(s);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  }, [setStatus, setError, startOperation, endOperation]);

  const panes: PaneDefinition[] = useMemo(() => {
    const result: PaneDefinition[] = [];

    for (let i = 0; i < filteredGroups.length; i++) {
      const { group, files } = filteredGroups[i];
      const offset = groupOffsets[i];
      const isIndex = group.kind === "index";
      const paths = files.map((f) => f.path);

      result.push({
        id: `group-${group.kind}`,
        defaultRatio: GROUP_RATIOS[group.kind] ?? 1,
        header: (collapsed, onToggle) => (
          <ResourceGroupHeader
            collapsed={collapsed}
            onToggle={onToggle}
            label={group.label}
            count={files.length}
            isIndex={isIndex}
            onStageAll={() => handleStageAll(paths)}
            onUnstageAll={handleUnstageAll}
            onDiscardAll={() => handleDiscardAll(files)}
          />
        ),
        body: () => (
          <ResourceGroupBody
            files={files}
            viewMode={viewMode}
            groupKind={group.kind}
            flatIndexOffset={offset}
            focusedIndex={focusedIndex}
          />
        ),
      });
    }

    if (stashes.length > 0) {
      result.push({
        id: "stashes",
        defaultRatio: 0.3,
        header: (collapsed, onToggle) => (
          <StashHeader collapsed={collapsed} onToggle={onToggle} count={stashes.length} />
        ),
        body: () => (
          <StashBody stashes={stashes} onApply={applyStash} onPop={popStash} onDrop={dropStash} />
        ),
      });
    }

    if (subRepos.length > 0 && status?.root) {
      result.push({
        id: "sub-repos",
        defaultRatio: 0.3,
        header: (collapsed, onToggle) => (
          <SubReposHeader collapsed={collapsed} onToggle={onToggle} count={subRepos.length} />
        ),
        body: () => (
          <SubReposBody repos={subRepos} repoRoot={status.root} />
        ),
      });
    }

    return result;
  }, [filteredGroups, groupOffsets, stashes, subRepos, status?.root, viewMode, focusedIndex, handleStageAll, handleUnstageAll, handleDiscardAll, applyStash, popStash, dropStash]);

  const hasFiles = status?.groups.some((g) => g.files.length > 0);

  return (
    <div className="scm-view">
      <div className="scm-header">
        <div className="scm-header-right">
          <ScmToolbar onOpenRepository={onOpenRepository} onCloneRepository={onCloneRepository} />
        </div>
      </div>
      <div className="scm-content">
        {status?.operationState && status.operationState !== "none" && (
          <MergeBanner operationState={status.operationState} />
        )}
        <CommitInput />
        {hasFiles && <FileFilter />}
        {status && status.groups.length === 0 && (
          <div className="scm-empty">
            <img
              className="scm-empty-watermark"
              src={isDark ? "/deathpush-white.png" : "/deathpush-black.png"}
              alt=""
            />
            <span className="scm-empty-label">No changes</span>
          </div>
        )}
        <ResizablePaneContainer panes={panes} />
        {!status && (
          <div className="scm-empty">
            <span style={{ opacity: 0.5, padding: 16, display: "block", textAlign: "center" }}>
              No repository open
            </span>
            <button
              className="action-button"
              style={{ margin: "0 16px", width: "auto" }}
              onClick={onOpenRepository}
            >
              <span className="codicon codicon-folder-opened" />
              Open Repository
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
