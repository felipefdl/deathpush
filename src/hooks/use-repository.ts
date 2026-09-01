import { repositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import * as commands from "../lib/tauri-commands";
import { replaceFromSnapshot } from "../stores/status-store";
import { beginRepositorySession } from "./use-repository-events";

const yieldToPaint = (): Promise<void> => {
  const { promise, resolve } = Promise.withResolvers<void>();
  requestAnimationFrame(() => {
    requestAnimationFrame(() => resolve());
  });
  return promise;
};

export const recoverFromSnapshot = async (): Promise<void> => {
  await commands.getStatus();
  const snapshot = await commands.getStatusSnapshot();
  replaceFromSnapshot(snapshot);
  const { applyMetadata, syncStatusGroups } = repositoryStore.getState();
  applyMetadata(snapshot.metadata);
  syncStatusGroups();
};

export const useRepository = () => {
  const openRepo = async (path: string) => {
    const { setIdentity, startOperation, endOperation, setError } = repositoryStore.getState();
    startOperation("open-repo");
    setError(null);
    await yieldToPaint();
    try {
      beginRepositorySession();
      const identity = await commands.openRepository(path);
      setIdentity(identity, { reset: false });
      addRecentProject(identity.root, identity.headBranch ?? undefined);
      void commands.getStatus().catch(() => undefined);
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
