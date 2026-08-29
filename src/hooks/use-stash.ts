import { repositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useStash = () => {
  const loadStashes = async () => {
    const { setStashes, setError } = repositoryStore.getState();
    try {
      const stashes = await commands.stashList();
      setStashes(stashes);
    } catch (err) {
      setError(String(err));
    }
  };

  const saveStash = async (message?: string) => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.stashSave(message);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  };

  const saveStashIncludeUntracked = async (message?: string) => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.stashSaveIncludeUntracked(message);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  };

  const saveStashStaged = async (message?: string) => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.stashSaveStaged(message);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  };

  const applyStash = async (index: number) => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.stashApply(index);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  };

  const popStash = async (index: number) => {
    const { setStatus, setError } = repositoryStore.getState();
    try {
      const status = await commands.stashPop(index);
      setStatus(status);
      await loadStashes();
    } catch (err) {
      setError(String(err));
    }
  };

  const dropStash = async (index: number) => {
    const { setStashes, setError } = repositoryStore.getState();
    try {
      const stashes = await commands.stashDrop(index);
      setStashes(stashes);
    } catch (err) {
      setError(String(err));
    }
  };

  const showStash = async (index: number) => {
    const { setError } = repositoryStore.getState();
    try {
      return await commands.stashShow(index);
    } catch (err) {
      setError(String(err));
      return null;
    }
  };

  return {
    loadStashes,
    saveStash,
    saveStashIncludeUntracked,
    saveStashStaged,
    applyStash,
    popStash,
    dropStash,
    showStash,
  };
};
