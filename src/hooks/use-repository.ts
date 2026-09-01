import { repositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import * as commands from "../lib/tauri-commands";
import { statusSession } from "../stores/status-store";
import { applyRecoveredSnapshot, beginRepositorySession } from "./use-repository-events";

const yieldToPaint = (): Promise<void> => {
  const { promise, resolve } = Promise.withResolvers<void>();
  requestAnimationFrame(() => {
    requestAnimationFrame(() => resolve());
  });
  return promise;
};

export const recoverFromSnapshot = async (): Promise<void> => {
  const session = statusSession();
  const root = repositoryStore.getState().status?.root ?? null;
  await commands.getStatus();
  const snapshot = await commands.getStatusSnapshot();
  applyRecoveredSnapshot(snapshot, session, root);
};

export const useRepository = () => {
  const openRepo = async (path: string) => {
    const { setIdentity, startOperation, endOperation, setError } = repositoryStore.getState();
    startOperation("open-repo");
    setError(null);
    await yieldToPaint();
    try {
      const identity = await commands.openRepository(path);
      beginRepositorySession();
      setIdentity(identity, { reset: false });
      addRecentProject(identity.root, identity.headBranch ?? undefined);
      void recoverFromSnapshot().catch(() => undefined);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("open-repo");
    }
  };

  const refreshStatus = async () => {
    const { setError } = repositoryStore.getState();
    try {
      await recoverFromSnapshot();
    } catch (err) {
      setError(String(err));
    }
  };

  return { openRepo, refreshStatus };
};
