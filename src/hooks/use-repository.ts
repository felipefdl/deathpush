import { repositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import { fetchSessionSnapshot, sendIntent } from "../lib/session-client";

const yieldToPaint = (): Promise<void> => {
  const { promise, resolve } = Promise.withResolvers<void>();
  requestAnimationFrame(() => {
    requestAnimationFrame(() => resolve());
  });
  return promise;
};

export const recoverFromSnapshot = async (): Promise<void> => {
  await fetchSessionSnapshot();
};

export const useRepository = () => {
  const openRepo = async (path: string) => {
    const { startOperation, endOperation, setError } = repositoryStore.getState();
    startOperation("open-repo");
    setError(null);
    await yieldToPaint();
    try {
      const result = await sendIntent({ type: "openRepository", path });
      if (result.kind === "snapshot") {
        addRecentProject(result.snapshot.repo.root, result.snapshot.repo.headBranch ?? undefined);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("open-repo");
    }
  };

  const refreshStatus = async () => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "refreshStatus" });
    } catch (err) {
      setError(String(err));
    }
  };

  return { openRepo, refreshStatus };
};
