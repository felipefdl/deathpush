import { createEffect, createMemo } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import type { FileEntry } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
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
import { flushPaths } from "../../lib/pierre/flush-registry";
import * as commands from "../../lib/tauri-commands";
import "../../styles/scm.css";
import "../../styles/repositories.css";

const GROUP_RATIOS: Record<string, number> = {
  merge: 1,
  index: 0.8,
  workingTree: 1,
  untracked: 0.4,
};

type ScmViewProps = {
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
};

export const ScmView = (props: ScmViewProps) => {
  const status = useStore(repositoryStore, (s) => s.status);
  const stashes = useStore(repositoryStore, (s) => s.stashes);
  const fileFilter = useStore(repositoryStore, (s) => s.fileFilter);
  const { setStatus, setError, startOperation, endOperation } = repositoryStore.getState();
  const colorScheme = useColorScheme();
  const isDark = () => colorScheme() === "dark";
  useGitStatus();
  const { loadStashes, applyStash, popStash, dropStash } = useStash();
  const { repos: subRepos } = useSubRepos();

  createEffect(
    () => status(),
    (current) => {
      if (current) {
        void loadStashes();
      }
    }
  );

  const filteredGroups = createMemo(() => {
    const current = status();
    if (!current) return [];
    const lower = fileFilter().toLowerCase();
    return current.groups
      .map((group) => {
        const files = fileFilter() ? group.files.filter((f) => f.path.toLowerCase().includes(lower)) : group.files;
        return { group, files };
      })
      .filter(({ files }) => files.length > 0);
  });

  const handleStageAll = async (paths: string[]) => {
    startOperation("stage");
    try {
      await flushPaths(paths);
      const s = await commands.stageFiles(paths);
      setStatus(s);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstageAll = async () => {
    startOperation("unstage");
    try {
      const s = await commands.unstageAll();
      setStatus(s);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscardAll = async (files: FileEntry[]) => {
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
      await flushPaths([...trackedPaths, ...untrackedPaths]);
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
  };

  const panes = createMemo((): PaneDefinition[] => {
    const result: PaneDefinition[] = [];
    const groups = filteredGroups();
    const stashList = stashes();
    const nested = subRepos();
    const root = status()?.root;

    for (let i = 0; i < groups.length; i++) {
      const { group, files } = groups[i];
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
        body: () => <ResourceGroupBody files={files} groupKind={group.kind} />,
      });
    }

    if (stashList.length > 0) {
      result.push({
        id: "stashes",
        defaultRatio: 0.3,
        header: (collapsed, onToggle) => (
          <StashHeader collapsed={collapsed} onToggle={onToggle} count={stashList.length} />
        ),
        body: () => <StashBody stashes={stashList} onApply={applyStash} onPop={popStash} onDrop={dropStash} />,
      });
    }

    if (nested.length > 0 && root) {
      result.push({
        id: "sub-repos",
        defaultRatio: 0.3,
        defaultCollapsed: true,
        header: (collapsed, onToggle) => (
          <SubReposHeader collapsed={collapsed} onToggle={onToggle} count={nested.length} />
        ),
        body: () => <SubReposBody repos={nested} repoRoot={root} />,
      });
    }

    return result;
  });

  const hasFiles = () => status()?.groups.some((g) => g.files.length > 0);
  const operationState = () => status()?.operationState;

  return (
    <div class="scm-view">
      <div class="scm-header">
        <div class="scm-header-right">
          <ScmToolbar onOpenRepository={props.onOpenRepository} onCloneRepository={props.onCloneRepository} />
        </div>
      </div>
      <div class="scm-content">
        {operationState() && operationState() !== "none" && <MergeBanner operationState={operationState()!} />}
        <CommitInput />
        {hasFiles() && <FileFilter />}
        {status() && status()!.groups.length === 0 && (
          <div class="scm-empty">
            <img class="scm-empty-watermark" src={isDark() ? "/deathpush-white.png" : "/deathpush-black.png"} alt="" />
            <span class="scm-empty-label">No changes</span>
          </div>
        )}
        <ResizablePaneContainer panes={panes()} />
        {!status() && (
          <div class="scm-empty">
            <span style={{ opacity: 0.5, padding: "16px", display: "block", "text-align": "center" }}>
              No repository open
            </span>
            <button
              class="action-button"
              style={{ margin: "0 16px", width: "auto" }}
              onClick={() => props.onOpenRepository()}
            >
              <span class="codicon codicon-folder-opened" />
              Open Repository
            </button>
          </div>
        )}
      </div>
    </div>
  );
};
