import { onSettled } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";
import { useCommitLog } from "../../hooks/use-commit-log";
import { CommitList } from "./commit-list";
import { CommitDetail } from "./commit-detail";
import { sendIntent } from "../../lib/session-client";

export const HistoryView = () => {
  const historyListWidth = useStore(layoutStore, (s) => s.historyListWidth);
  const fileHistoryPath = useStore(repositoryStore, (s) => s.fileHistoryPath);
  const { loadMore, selectCommit } = useCommitLog();

  onSettled(() => {
    const handler = (e: Event) => {
      const path = (e as CustomEvent<{ path: string }>).detail.path;
      void sendIntent({ type: "openFileHistory", path }).catch((err: unknown) => {
        repositoryStore.getState().setError(String(err));
      });
    };
    window.addEventListener("deathpush:file-history", handler);
    return () => window.removeEventListener("deathpush:file-history", handler);
  });

  const handleClearFileHistory = () => {
    void sendIntent({ type: "clearFileHistory" }).catch((err: unknown) => {
      repositoryStore.getState().setError(String(err));
    });
  };

  const handleDividerMouseDown = (e: MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = historyListWidth();

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(200, Math.min(600, startWidth + (moveEvent.clientX - startX)));
      layoutStore.getState().setHistoryListWidth(newWidth);
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  return (
    <div class="history-view">
      <div class="history-list-panel" style={{ width: `${historyListWidth()}px` }}>
        {fileHistoryPath() && (
          <div class="file-history-header">
            <span class="codicon codicon-history" />
            <span class="file-history-path" title={fileHistoryPath()!}>
              {fileHistoryPath()!.split("/").pop()}
            </span>
            <div style={{ flex: 1 }} />
            <button class="scm-toolbar-button" onClick={handleClearFileHistory} title="Show full history">
              <span class="codicon codicon-close" />
            </button>
          </div>
        )}
        <CommitList onLoadMore={loadMore} onSelectCommit={selectCommit} />
      </div>
      <div class="history-divider" onMouseDown={handleDividerMouseDown} />
      <div class="history-detail-panel">
        <CommitDetail />
      </div>
    </div>
  );
};
