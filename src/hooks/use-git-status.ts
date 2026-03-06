import { useCallback } from "react";
import { useRepository } from "./use-repository";
import { useDiff } from "./use-diff";
import { useRepositoryStore } from "../stores/repository-store";
import { useTauriEvent } from "./use-tauri-event";

export const useGitStatus = () => {
  const { refreshStatus } = useRepository();
  const { loadDiff } = useDiff();

  const handleChange = useCallback(() => {
    refreshStatus();
    const { selectedFile } = useRepositoryStore.getState();
    if (selectedFile) {
      loadDiff(selectedFile.path, selectedFile.staged);
    }
  }, [refreshStatus, loadDiff]);

  useTauriEvent("repository-changed", handleChange);

  return { refreshStatus };
};
