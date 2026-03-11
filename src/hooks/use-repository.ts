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
