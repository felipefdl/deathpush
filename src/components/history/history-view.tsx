import { createEffect, createSignal, onSettled } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";
import { useCommitLog } from "../../hooks/use-commit-log";
import { CommitList } from "./commit-list";
import { CommitDetail } from "./commit-detail";
import * as commands from "../../lib/tauri-commands";
import type { CommitEntry } from "../../lib/git-types";

const FILE_HISTORY_PAGE_SIZE = 50;

export const HistoryView = () => {
  const status = useStore(repositoryStore, (s) => s.status);
  const historyListWidth = useStore(layoutStore, (s) => s.historyListWidth);
  const { loadCommitLog, loadMore, selectCommit } = useCommitLog();
  const [fileHistoryPath, setFileHistoryPath] = createSignal<string | null>(null);

  const loadFileHistory = async (path: string, reset: boolean = true) => {
    const { setCommitLog, setError } = repositoryStore.getState();
    try {
      const currentLog = repositoryStore.getState().commitLog;
      const skip = reset ? 0 : currentLog.length;
      const entries: CommitEntry[] = await commands.getFileLog(path, skip, FILE_HISTORY_PAGE_SIZE);
      if (reset) {
        setCommitLog(entries);
      } else {
        setCommitLog([...currentLog, ...entries]);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  createEffect(
    () => status()?.headCommit,
    () => {
      if (repositoryStore.getState().status && !fileHistoryPath()) {
        void loadCommitLog(true);
      }
    }
  );

  onSettled(() => {
    const handler = (e: Event) => {
      const path = (e as CustomEvent<{ path: string }>).detail.path;
      setFileHistoryPath(path);
      void loadFileHistory(path, true);
    };
    window.addEventListener("deathpush:file-history", handler);
    return () => window.removeEventListener("deathpush:file-history", handler);
  });

  const handleClearFileHistory = () => {
    setFileHistoryPath(null);
    void loadCommitLog(true);
  };

  const handleLoadMore = () => {
    const path = fileHistoryPath();
    if (path) {
      void loadFileHistory(path, false);
    } else {
      void loadMore();
    }
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
        <CommitList onLoadMore={handleLoadMore} onSelectCommit={selectCommit} />
      </div>
      <div class="history-divider" onMouseDown={handleDividerMouseDown} />
      <div class="history-detail-panel">
        <CommitDetail />
      </div>
    </div>
  );
};
