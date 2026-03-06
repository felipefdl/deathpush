import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useStash = () => {
  const { setStashes, setStatus, setError } = useRepositoryStore();

  const loadStashes = useCallback(async () => {
    try {
      const stashes = await commands.stashList();
      setStashes(stashes);
    } catch (err) {
      setError(String(err));
    }
  }, [setStashes, setError]);

  const saveStash = useCallback(async (message?: string) => {
    try {
      const status = await commands.stashSave(message);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError, loadStashes]);

  const applyStash = useCallback(async (index: number) => {
    try {
      const status = await commands.stashApply(index);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const popStash = useCallback(async (index: number) => {
    try {
      const status = await commands.stashPop(index);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError, loadStashes]);

  const dropStash = useCallback(async (index: number) => {
    try {
      const stashes = await commands.stashDrop(index);
      setStashes(stashes);
    } catch (err) {
      setError(String(err));
    }
  }, [setStashes, setError]);

  return { loadStashes, saveStash, applyStash, popStash, dropStash };
};
