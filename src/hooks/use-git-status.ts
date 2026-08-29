import { useRepository } from "./use-repository";
import { useDiff } from "./use-diff";
import { repositoryStore } from "../stores/repository-store";
import { useTauriEvent } from "./use-tauri-event";
import { throttle } from "../lib/throttle";

export const useGitStatus = () => {
  const { refreshStatus } = useRepository();
  const { loadDiff } = useDiff();

  const handleChange = throttle(() => {
    void refreshStatus();
    const { selectedFile } = repositoryStore.getState();
    if (selectedFile) {
      void loadDiff(selectedFile.path, selectedFile.staged);
    }
  }, 1000);

  useTauriEvent("repository-changed", handleChange);

  return { refreshStatus };
};
