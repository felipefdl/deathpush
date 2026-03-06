import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

const PAGE_SIZE = 50;

export const useCommitLog = () => {
  const { commitLog, setCommitLog, setSelectedCommit, setCommitDetail, setError } = useRepositoryStore();

  const loadCommitLog = useCallback(async (reset: boolean = true) => {
    try {
      const skip = reset ? 0 : commitLog.length;
      const entries = await commands.getCommitLog(skip, PAGE_SIZE);
      if (reset) {
        setCommitLog(entries);
      } else {
        setCommitLog([...commitLog, ...entries]);
      }
    } catch (err) {
      setError(String(err));
    }
  }, [commitLog, setCommitLog, setError]);

  const loadMore = useCallback(async () => {
    await loadCommitLog(false);
  }, [loadCommitLog]);

  const selectCommit = useCallback(async (id: string) => {
    setSelectedCommit(id);
    setCommitDetail(null);
    try {
      const detail = await commands.getCommitDetail(id);
      setCommitDetail(detail);
    } catch (err) {
      setError(String(err));
    }
  }, [setSelectedCommit, setCommitDetail, setError]);

  return { loadCommitLog, loadMore, selectCommit };
};
