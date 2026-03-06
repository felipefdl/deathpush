import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import { useLayoutStore } from "../stores/layout-store";
import * as commands from "../lib/tauri-commands";

export const useDiff = () => {
  const { setDiff, setSelectedFile, setError } = useRepositoryStore();
  const { mainView, setMainView } = useLayoutStore();

  const loadDiff = useCallback(async (path: string, staged: boolean) => {
    if (mainView !== "changes" && mainView !== "history") {
      setMainView("changes");
    }
    setSelectedFile({ path, staged });
    try {
      const diff = await commands.getFileDiff(path, staged);
      setDiff(diff);
    } catch (err) {
      setError(String(err));
      setDiff(null);
    }
  }, [mainView, setMainView, setDiff, setSelectedFile, setError]);

  const clearDiff = useCallback(() => {
    setSelectedFile(null);
    setDiff(null);
  }, [setDiff, setSelectedFile]);

  return { loadDiff, clearDiff };
};
