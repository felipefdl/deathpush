import { repositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

const PAGE_SIZE = 50;

export const useCommitLog = () => {
  const loadCommitLog = async (reset: boolean = true) => {
    const { commitLog, setCommitLog, setError } = repositoryStore.getState();
    try {
      const skip = reset ? 0 : commitLog.length;
      const entries = await commands.getCommitLog(skip, PAGE_SIZE);
      if (reset) {
        setCommitLog(entries);
      } else {
        setCommitLog([...repositoryStore.getState().commitLog, ...entries]);
      }
    } catch (err) {
      setError(String(err));
    }
  };

  const loadMore = async () => {
    await loadCommitLog(false);
  };

  const selectCommit = async (id: string) => {
    const { setSelectedCommit, setCommitDetail, setError } = repositoryStore.getState();
    setSelectedCommit(id);
    setCommitDetail(null);
    try {
      const detail = await commands.getCommitDetail(id);
      setCommitDetail(detail);
    } catch (err) {
      setError(String(err));
    }
  };

  return { loadCommitLog, loadMore, selectCommit };
};
