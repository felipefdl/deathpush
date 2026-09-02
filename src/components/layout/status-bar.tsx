import { createMemo, createSignal } from "solid-js";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { BranchPicker } from "../branch/branch-picker";
import { formatRelativeDate } from "../../lib/format-date";

export const StatusBar = () => {
  const status = useStore(repositoryStore, (s) => s.status);
  const blame = useStore(repositoryStore, (s) => s.blame);
  const cursorLine = useStore(repositoryStore, (s) => s.cursorLine);
  const blameEnabled = useStore(settingsStore, (s) => s.settings.git.blame);
  const zoomLevel = useStore(settingsStore, (s) => s.settings.ui.zoomLevel);
  const lastCommit = useStore(repositoryStore, (s) => s.lastCommit);
  const zoomPercent = createMemo(() => (zoomLevel() !== 0 ? `${Math.round(Math.pow(1.2, zoomLevel()) * 100)}%` : null));
  const [showBranchPicker, setShowBranchPicker] = createSignal(false);

  const branch = () => status()?.headBranch ?? "No branch";
  const ahead = () => status()?.ahead ?? 0;
  const behind = () => status()?.behind ?? 0;

  const syncLabel = () =>
    ahead() > 0 || behind() > 0
      ? `${behind() > 0 ? `${behind()}\u2193 ` : ""}${ahead() > 0 ? `${ahead()}\u2191` : ""}`
      : "";


  const cursorBlame = createMemo(() => {
    const blameData = blame();
    const line = cursorLine();
    if (!blameEnabled() || !blameData || line === null) return null;
    const group = blameData.lineGroups.find((g) => line >= g.startLine && line <= g.endLine);
    if (!group || group.commitId.startsWith("0000000")) return null;
    return `${group.authorName}, ${formatRelativeDate(group.authorDate)} - ${group.summary}`;
  });

  return (
    <>
      <div class="status-bar">
        <button class="status-bar-item" onClick={() => setShowBranchPicker(true)} title="Switch branch">
          <span class="codicon codicon-source-control" />
          <span class="status-bar-text">{branch()}</span>
          {syncLabel() && <span class="status-bar-text">{syncLabel()}</span>}
        </button>
        {cursorBlame() && (
          <span class="status-bar-item status-bar-blame" title={cursorBlame()!}>
            <span class="codicon codicon-person" />
            <span class="status-bar-text">{cursorBlame()}</span>
          </span>
        )}
        <div class="status-bar-spacer" />
        {zoomPercent() && (
          <button class="status-bar-item" onClick={() => settingsStore.getState().resetZoom()} title="Reset Zoom">
            <span class="codicon codicon-zoom-in" />
            <span class="status-bar-text">{zoomPercent()}</span>
          </button>
        )}
        {lastCommit() && (
          <button
            class="status-bar-item"
            title="View history"
            onClick={() => layoutStore.getState().setMainView("history")}
          >
            <span class="codicon codicon-git-commit" />
            <span class="status-bar-text status-bar-last-commit">{lastCommit()!.message}</span>
            <span class="status-bar-text" style={{ opacity: 0.7 }}>
              {formatRelativeDate(lastCommit()!.authorDate)}
            </span>
          </button>
        )}
      </div>
      {showBranchPicker() && <BranchPicker onClose={() => setShowBranchPicker(false)} />}
    </>
  );
};
