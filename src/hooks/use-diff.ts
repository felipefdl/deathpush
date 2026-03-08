import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

const isDiffEqual = (
  a: { path: string; original: string; modified: string; fileType: string } | null,
  b: { path: string; original: string; modified: string; fileType: string } | null,
) => {
  if (a === b) return true;
  if (!a || !b) return false;
  return a.path === b.path && a.original === b.original && a.modified === b.modified && a.fileType === b.fileType;
};

export const useDiff = () => {
  const { setDiff, setSelectedFile, setError } = useRepositoryStore();

  const loadDiff = useCallback(async (path: string, staged: boolean) => {
    setSelectedFile({ path, staged });
    try {
      const diff = await commands.getFileDiff(path, staged);
      const current = useRepositoryStore.getState().diff;
      if (!isDiffEqual(current, diff)) {
        setDiff(diff);
      }
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
