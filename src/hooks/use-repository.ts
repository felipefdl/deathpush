import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import { addRecentProject } from "../lib/recent-projects";
import * as commands from "../lib/tauri-commands";

export const useRepository = () => {
  const { setStatus, startOperation, endOperation, setError } = useRepositoryStore();

  const openRepo = useCallback(async (path: string) => {
    startOperation("open-repo");
    setError(null);
    try {
      const status = await commands.openRepository(path);
      setStatus(status);
      addRecentProject(status.root);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("open-repo");
    }
  }, [setStatus, startOperation, endOperation, setError]);

  const refreshStatus = useCallback(async () => {
    try {
      const status = await commands.getStatus();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  return { openRepo, refreshStatus };
};
