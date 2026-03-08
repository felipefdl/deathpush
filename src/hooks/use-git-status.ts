import { useMemo } from "react";
import { useRepository } from "./use-repository";
import { useDiff } from "./use-diff";
import { useRepositoryStore } from "../stores/repository-store";
import { useTauriEvent } from "./use-tauri-event";
import { throttle } from "../lib/throttle";

export const useGitStatus = () => {
  const { refreshStatus } = useRepository();
  const { loadDiff } = useDiff();

  const handleChange = useMemo(
    () =>
      throttle(() => {
        refreshStatus();
        const { selectedFile } = useRepositoryStore.getState();
        if (selectedFile) {
          loadDiff(selectedFile.path, selectedFile.staged);
        }
      }, 1000),
    [refreshStatus, loadDiff],
  );

  useTauriEvent("repository-changed", handleChange);

  return { refreshStatus };
};
