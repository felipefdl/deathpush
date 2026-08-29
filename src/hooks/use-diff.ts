import type { ResourceGroupKind } from "../lib/git-types";
import { repositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

const isDiffEqual = (
  a: { path: string; original: string; modified: string; fileType: string } | null,
  b: { path: string; original: string; modified: string; fileType: string } | null
) => {
  if (a === b) return true;
  if (!a || !b) return false;
  return a.path === b.path && a.original === b.original && a.modified === b.modified && a.fileType === b.fileType;
};

export const useDiff = () => {
  const loadDiff = async (path: string, staged: boolean, groupKind: ResourceGroupKind = "workingTree") => {
    const { setDiff, setSelectedFile, setError } = repositoryStore.getState();
    setSelectedFile({ path, staged, groupKind });
    try {
      const diff = await commands.getFileDiff(path, staged);
      const current = repositoryStore.getState().diff;
      if (!isDiffEqual(current, diff)) {
        setDiff(diff);
      }
    } catch (err) {
      setError(String(err));
      setDiff(null);
    }
  };

  const clearDiff = () => {
    const { setDiff, setSelectedFile } = repositoryStore.getState();
    setSelectedFile(null);
    setDiff(null);
  };

  return { loadDiff, clearDiff };
};
