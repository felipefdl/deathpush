import { repositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import * as commands from "../lib/tauri-commands";

const yieldToPaint = (): Promise<void> => {
  const { promise, resolve } = Promise.withResolvers<void>();
  requestAnimationFrame(() => {
    requestAnimationFrame(() => resolve());
  });
  return promise;
};

export const useRepository = () => {
  const openRepo = async (path: string) => {
    const { setStatus, startOperation, endOperation, setError } = repositoryStore.getState();
    startOperation("open-repo");
    setError(null);
    await yieldToPaint();
    try {
      const basicStatus = await commands.openRepository(path);
      addRecentProject(basicStatus.root);
      try {
        const fullStatus = await commands.getStatus();
        setStatus(fullStatus);
      } catch {
        setStatus(basicStatus);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("open-repo");
    }
  };

  const refreshStatus = async () => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.getStatus();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  };

  return { openRepo, refreshStatus };
};
