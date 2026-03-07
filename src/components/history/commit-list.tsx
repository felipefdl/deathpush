import { useCallback, useState } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../../stores/repository-store";
import { formatRelativeDate } from "../../lib/format-date";
import { getAuthorInitials, hashAuthorColor } from "../../lib/author-utils";
import * as commands from "../../lib/tauri-commands";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";
import type { CommitEntry } from "../../lib/git-types";

const failedAvatarUrls = new Set<string>();

const getGitHubAvatarUrl = (email: string): string | null => {
  const lower = email.trim().toLowerCase();
  if (lower.endsWith("@users.noreply.github.com")) {
    const local = lower.split("@")[0];
    const username = local.includes("+") ? local.split("+")[1] : local;
    if (username) return `https://github.com/${username}.png?size=48`;
  }
  return `https://avatars.githubusercontent.com/u/e?email=${encodeURIComponent(lower)}&s=48`;
};

const AuthorAvatar = ({ entry }: { entry: CommitEntry }) => {
  const githubUrl = getGitHubAvatarUrl(entry.authorEmail);
  const gravatarUrl = entry.avatarUrl;
  const primaryUrl = githubUrl ?? gravatarUrl;
  const fallbackUrl = githubUrl ? gravatarUrl : null;
  const [src, setSrc] = useState(() => {
    if (failedAvatarUrls.has(primaryUrl)) {
      return fallbackUrl && !failedAvatarUrls.has(fallbackUrl) ? fallbackUrl : null;
    }
    return primaryUrl;
  });

  if (!src) {
    return (
      <span
        className="commit-avatar"
        style={{ backgroundColor: hashAuthorColor(entry.authorName) }}
      >
        {getAuthorInitials(entry.authorName)}
      </span>
    );
  }

  return (
    <img
      className="commit-avatar"
      src={src}
      alt=""
      onError={() => {
        failedAvatarUrls.add(src);
        if (src === primaryUrl && fallbackUrl && !failedAvatarUrls.has(fallbackUrl)) {
          setSrc(fallbackUrl);
        } else {
          setSrc(null);
        }
      }}
    />
  );
};

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
    return (
      <div className="history-empty">
        <span className="codicon codicon-git-commit history-empty-icon" />
        <span>No commits found</span>
      </div>
    );
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
            <AuthorAvatar entry={entry} />
            <div className="commit-list-item-content">
              <div className="commit-list-item-top">
                <span className="commit-list-item-message" title={entry.message}>
                  {firstLine}
                </span>
                <span className="commit-list-item-date">{formatRelativeDate(entry.authorDate)}</span>
              </div>
              <div className="commit-list-item-bottom">
                <span className="commit-list-item-id">{entry.shortId}</span>
                {entry.parentIds.length > 1 && (
                  <span className="commit-merge-badge" title="Merge commit">
                    <span className="codicon codicon-git-merge" />
                  </span>
                )}
                <span className="commit-list-item-author">{entry.authorName}</span>
              </div>
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
