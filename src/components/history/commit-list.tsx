import { createSignal, For } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
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

type AuthorAvatarProps = {
  entry: CommitEntry;
};

const AuthorAvatar = (props: AuthorAvatarProps) => {
  const githubUrl = getGitHubAvatarUrl(props.entry.authorEmail);
  const gravatarUrl = props.entry.avatarUrl;
  const primaryUrl = githubUrl ?? gravatarUrl;
  const fallbackUrl = githubUrl ? gravatarUrl : null;
  const initialSrc = failedAvatarUrls.has(primaryUrl)
    ? fallbackUrl && !failedAvatarUrls.has(fallbackUrl)
      ? fallbackUrl
      : null
    : primaryUrl;
  const [src, setSrc] = createSignal<string | null>(initialSrc);

  return (
    <>
      {!src() ? (
        <span class="commit-avatar" style={{ "background-color": hashAuthorColor(props.entry.authorName) }}>
          {getAuthorInitials(props.entry.authorName)}
        </span>
      ) : (
        <img
          class="commit-avatar"
          src={src()!}
          alt=""
          onError={() => {
            const current = src();
            if (!current) return;
            failedAvatarUrls.add(current);
            if (current === primaryUrl && fallbackUrl && !failedAvatarUrls.has(fallbackUrl)) {
              setSrc(fallbackUrl);
            } else {
              setSrc(null);
            }
          }}
        />
      )}
    </>
  );
};

type CommitListProps = {
  onLoadMore: () => void;
  onSelectCommit: (id: string) => void;
};

export const CommitList = (props: CommitListProps) => {
  const commitLog = useStore(repositoryStore, (s) => s.commitLog);
  const selectedCommit = useStore(repositoryStore, (s) => s.selectedCommit);
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number; commitId: string } | null>(null);

  const handleCherryPick = async (commitId: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.cherryPick(commitId);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleReset = async (commitId: string, mode: string) => {
    if (mode === "hard") {
      const confirmed = await confirm(
        "Are you sure you want to hard reset? This will discard all uncommitted changes.\n\nThis action is irreversible.",
        { title: "Hard Reset", kind: "warning", okLabel: "Reset", cancelLabel: "Cancel" }
      );
      if (!confirmed) return;
    }
    const { setError } = repositoryStore.getState();
    try {
      await commands.resetToCommit(commitId, mode);
    } catch (err) {
      setError(String(err));
    }
  };

  const handleContextMenu = (e: MouseEvent, commitId: string) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY, commitId });
  };

  const handleCopyCommitId = (commitId: string) => {
    void navigator.clipboard.writeText(commitId);
  };

  const handleCopyCommitMessage = (commitId: string) => {
    const entry = repositoryStore.getState().commitLog.find((item) => item.id === commitId);
    if (entry) void navigator.clipboard.writeText(entry.message);
  };

  const getContextMenuItems = (commitId: string): ContextMenuItem[] => {
    const entry = repositoryStore.getState().commitLog.find((item) => item.id === commitId);
    const shortId = entry?.shortId ?? commitId.slice(0, 7);
    return [
      { label: `Copy Commit ID (${shortId})`, icon: "copy", action: () => handleCopyCommitId(commitId) },
      { label: "Copy Commit Message", icon: "copy", action: () => handleCopyCommitMessage(commitId) },
      { label: "", action: () => {}, separator: true },
      { label: "Cherry-pick Commit", icon: "git-commit", action: () => handleCherryPick(commitId) },
      { label: "", action: () => {}, separator: true },
      { label: "Reset (Soft)", icon: "history", action: () => handleReset(commitId, "soft") },
      { label: "Reset (Mixed)", icon: "history", action: () => handleReset(commitId, "mixed") },
      { label: "Reset (Hard)", icon: "warning", action: () => handleReset(commitId, "hard") },
    ];
  };

  return (
    <>
      {commitLog().length === 0 ? (
        <div class="history-empty">
          <span class="codicon codicon-git-commit history-empty-icon" />
          <span>No commits found</span>
        </div>
      ) : (
        <div class="commit-list">
          <For each={commitLog()} keyed={(entry) => entry.id}>
            {(entry) => {
              const firstLine = () => entry().message.split("\n")[0];
              return (
                <div
                  class={["commit-list-item", { selected: selectedCommit() === entry().id }]}
                  onClick={() => props.onSelectCommit(entry().id)}
                  onContextMenu={(e) => handleContextMenu(e, entry().id)}
                >
                  <AuthorAvatar entry={entry()} />
                  <div class="commit-list-item-content">
                    <div class="commit-list-item-top">
                      <span class="commit-list-item-message" title={entry().message}>
                        {firstLine()}
                      </span>
                      <span class="commit-list-item-date">{formatRelativeDate(entry().authorDate)}</span>
                    </div>
                    <div class="commit-list-item-bottom">
                      <span class="commit-list-item-id">{entry().shortId}</span>
                      {entry().parentIds.length > 1 && (
                        <span class="commit-merge-badge" title="Merge commit">
                          <span class="codicon codicon-git-merge" />
                        </span>
                      )}
                      <span class="commit-list-item-author">{entry().authorName}</span>
                    </div>
                  </div>
                </div>
              );
            }}
          </For>
          <button class="commit-list-load-more" onClick={() => props.onLoadMore()}>
            Load More
          </button>
          {contextMenu() && (
            <ContextMenu
              x={contextMenu()!.x}
              y={contextMenu()!.y}
              items={getContextMenuItems(contextMenu()!.commitId)}
              onClose={() => setContextMenu(null)}
            />
          )}
        </div>
      )}
    </>
  );
};
