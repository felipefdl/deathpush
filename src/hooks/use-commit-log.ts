import { repositoryStore } from "../stores/repository-store";
import { sendIntent } from "../lib/session-client";

export const useCommitLog = () => {
  const loadMore = async () => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "loadMoreLog" });
    } catch (err) {
      setError(String(err));
    }
  };

  const selectCommit = async (id: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "selectCommit", id });
    } catch (err) {
      setError(String(err));
    }
  };

  return { loadMore, selectCommit };
};
