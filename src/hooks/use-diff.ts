import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useDiff = () => {
  const { setDiff, setSelectedFile, setError } = useRepositoryStore();

  const loadDiff = useCallback(async (path: string, staged: boolean) => {
    setSelectedFile({ path, staged });
    try {
      const diff = await commands.getFileDiff(path, staged);
      setDiff(diff);
    } catch (err) {
      setError(String(err));
      setDiff(null);
    }
  }, [setDiff, setSelectedFile, setError]);

  const clearDiff = useCallback(() => {
    setSelectedFile(null);
    setDiff(null);
  }, [setDiff, setSelectedFile]);

  return { loadDiff, clearDiff };
};
