import { useCallback, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../../stores/repository-store";
import { formatRelativeDate } from "../../lib/format-date";
import * as commands from "../../lib/tauri-commands";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";

interface CommitListProps {
  onLoadMore: () => void;
  onSelectCommit: (id: string) => void;
}

export const CommitList = ({ onLoadMore, onSelectCommit }: CommitListProps) => {
  const { commitLog, selectedCommit, setStatus, setError } = useRepositoryStore();
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; commitId: string } | null>(null);

  const handleCherryPick = useCallback(async (commitId: string) => {
    try {
      const status = await commands.cherryPick(commitId);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const handleReset = useCallback(async (commitId: string, mode: string) => {
    if (mode === "hard") {
      const confirmed = await confirm(
        "Are you sure you want to hard reset? This will discard all uncommitted changes.\n\nThis action is irreversible.",
        { title: "Hard Reset", kind: "warning", okLabel: "Reset", cancelLabel: "Cancel" },
      );
      if (!confirmed) return;
    }
    try {
      const status = await commands.resetToCommit(commitId, mode);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const handleContextMenu = useCallback((e: React.MouseEvent, commitId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, commitId });
  }, []);

  const getContextMenuItems = (commitId: string): ContextMenuItem[] => [
    { label: "Cherry-pick Commit", icon: "git-commit", action: () => handleCherryPick(commitId) },
    { label: "", action: () => {}, separator: true },
    { label: "Reset (Soft)", icon: "history", action: () => handleReset(commitId, "soft") },
    { label: "Reset (Mixed)", icon: "history", action: () => handleReset(commitId, "mixed") },
    { label: "Reset (Hard)", icon: "warning", action: () => handleReset(commitId, "hard") },
  ];

  if (commitLog.length === 0) {
    return <div className="history-empty">No commits found.</div>;
  }

  return (
    <div className="commit-list">
      {commitLog.map((entry) => {
        const firstLine = entry.message.split("\n")[0];
        const isSelected = selectedCommit === entry.id;
        return (
          <div
            key={entry.id}
            className={`commit-list-item${isSelected ? " selected" : ""}`}
            onClick={() => onSelectCommit(entry.id)}
            onContextMenu={(e) => handleContextMenu(e, entry.id)}
          >
            <div className="commit-list-item-top">
              <span className="commit-list-item-message" title={entry.message}>
                {firstLine}
              </span>
              <span className="commit-list-item-date">{formatRelativeDate(entry.authorDate)}</span>
            </div>
            <div className="commit-list-item-bottom">
              <span className="commit-list-item-id">{entry.shortId}</span>
              <span className="commit-list-item-author">{entry.authorName}</span>
            </div>
          </div>
        );
      })}
      <button className="commit-list-load-more" onClick={onLoadMore}>
        Load More
      </button>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getContextMenuItems(contextMenu.commitId)}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
};
