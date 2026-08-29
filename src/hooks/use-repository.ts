import { repositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import * as commands from "../lib/tauri-commands";

export const useRepository = () => {
  const openRepo = async (path: string) => {
    const { setStatus, startOperation, endOperation, setError } = repositoryStore.getState();
    startOperation("open-repo");
    setError(null);
    try {
      // Phase 1: fast open -- returns basic metadata with empty file groups
      const basicStatus = await commands.openRepository(path);
      setStatus(basicStatus);
      addRecentProject(basicStatus.root);
      endOperation("open-repo");

      // Phase 2: background full status -- populates file lists
      try {
        const fullStatus = await commands.getStatus();
        setStatus(fullStatus);
      } catch {
        // Non-critical: file watcher will trigger a refresh eventually
      }
    } catch (err) {
      setError(String(err));
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
