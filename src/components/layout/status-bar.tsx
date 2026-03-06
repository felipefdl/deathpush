import { useRepositoryStore } from "../../stores/repository-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useEffect, useMemo, useState } from "react";
import { BranchPicker } from "../branch/branch-picker";
import { formatRelativeDate } from "../../lib/format-date";
import * as commands from "../../lib/tauri-commands";
import type { LastCommitInfo } from "../../lib/git-types";

export const StatusBar = () => {
  const { status, blame, cursorLine } = useRepositoryStore();
  const blameEnabled = useSettingsStore((s) => s.settings.git.blame);
  const [showBranchPicker, setShowBranchPicker] = useState(false);
  const [lastCommit, setLastCommit] = useState<LastCommitInfo | null>(null);

  const branch = status?.headBranch ?? "No branch";
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;

  const syncLabel = ahead > 0 || behind > 0
    ? `${behind > 0 ? `${behind}\u2193 ` : ""}${ahead > 0 ? `${ahead}\u2191` : ""}`
    : "";

  useEffect(() => {
    if (!status?.headCommit) {
      setLastCommit(null);
      return;
    }
    commands.getLastCommitInfo().then(setLastCommit).catch(() => setLastCommit(null));
  }, [status?.headCommit]);

  const cursorBlame = useMemo(() => {
    if (!blameEnabled || !blame || cursorLine === null) return null;
    const group = blame.lineGroups.find(
      (g) => cursorLine >= g.startLine && cursorLine <= g.endLine,
    );
    if (!group || group.commitId.startsWith("0000000")) return null;
    return `${group.authorName}, ${formatRelativeDate(group.authorDate)} - ${group.summary}`;
  }, [blame, blameEnabled, cursorLine]);

  return (
    <>
      <div className="status-bar">
        <button
          className="status-bar-item"
          onClick={() => setShowBranchPicker(true)}
          title="Switch branch"
        >
          <span className="codicon codicon-source-control" />
          <span className="status-bar-text">{branch}</span>
          {syncLabel && <span className="status-bar-text">{syncLabel}</span>}
        </button>
        {cursorBlame && (
          <span className="status-bar-item status-bar-blame" title={cursorBlame}>
            <span className="codicon codicon-person" />
            <span className="status-bar-text">{cursorBlame}</span>
          </span>
        )}
        <div className="status-bar-spacer" />
        {lastCommit && (
          <span className="status-bar-item" title={`${lastCommit.shortId}: ${lastCommit.message}`}>
            <span className="codicon codicon-git-commit" />
            <span className="status-bar-text status-bar-last-commit">{lastCommit.message}</span>
            <span className="status-bar-text" style={{ opacity: 0.7 }}>
              {formatRelativeDate(lastCommit.authorDate)}
            </span>
          </span>
        )}
      </div>
      {showBranchPicker && (
        <BranchPicker onClose={() => setShowBranchPicker(false)} />
      )}
    </>
  );
};
