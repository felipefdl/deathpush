import { useCallback, useEffect, useState } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useCommitLog } from "../../hooks/use-commit-log";
import { CommitList } from "./commit-list";
import { CommitDetail } from "./commit-detail";
import * as commands from "../../lib/tauri-commands";
import type { CommitEntry } from "../../lib/git-types";

const FILE_HISTORY_PAGE_SIZE = 50;

export const HistoryView = () => {
  const { status, setCommitLog, setError } = useRepositoryStore();
  const { loadCommitLog, loadMore, selectCommit } = useCommitLog();
  const [fileHistoryPath, setFileHistoryPath] = useState<string | null>(null);

  useEffect(() => {
    if (status && !fileHistoryPath) {
      loadCommitLog(true);
    }
  }, [status?.headCommit, fileHistoryPath]);

  const loadFileHistory = useCallback(async (path: string, reset: boolean = true) => {
    try {
      const currentLog = useRepositoryStore.getState().commitLog;
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
  }, [setCommitLog, setError]);

  useEffect(() => {
    const handler = (e: Event) => {
      const path = (e as CustomEvent<{ path: string }>).detail.path;
      setFileHistoryPath(path);
      loadFileHistory(path, true);
    };
    window.addEventListener("deathpush:file-history", handler);
    return () => window.removeEventListener("deathpush:file-history", handler);
  }, [loadFileHistory]);

  const handleClearFileHistory = useCallback(() => {
    setFileHistoryPath(null);
    loadCommitLog(true);
  }, [loadCommitLog]);

  const handleLoadMore = useCallback(() => {
    if (fileHistoryPath) {
      loadFileHistory(fileHistoryPath, false);
    } else {
      loadMore();
    }
  }, [fileHistoryPath, loadFileHistory, loadMore]);

  return (
    <div className="history-view">
      <div className="history-list-panel">
        {fileHistoryPath && (
          <div className="file-history-header">
            <span className="codicon codicon-history" />
            <span className="file-history-path" title={fileHistoryPath}>
              {fileHistoryPath.split("/").pop()}
            </span>
            <div style={{ flex: 1 }} />
            <button
              className="scm-toolbar-button"
              onClick={handleClearFileHistory}
              title="Show full history"
            >
              <span className="codicon codicon-close" />
            </button>
          </div>
        )}
        <CommitList onLoadMore={handleLoadMore} onSelectCommit={selectCommit} />
      </div>
      <div className="history-detail-panel">
        <CommitDetail />
      </div>
    </div>
  );
};
