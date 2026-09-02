import { repositoryStore } from "../stores/repository-store";
import { sendDestructiveIntent, sendIntent } from "../lib/session-client";

export const useStash = () => {
  const saveStash = async (message?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({
        type: "stashSave",
        includeUntracked: false,
        stagedOnly: false,
        message: message ?? null,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  const saveStashIncludeUntracked = async (message?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({
        type: "stashSave",
        includeUntracked: true,
        stagedOnly: false,
        message: message ?? null,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  const saveStashStaged = async (message?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({
        type: "stashSave",
        includeUntracked: false,
        stagedOnly: true,
        message: message ?? null,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  const applyStash = async (index: number) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "stashApply", index });
    } catch (err) {
      setError(String(err));
    }
  };

  const popStash = async (index: number) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "stashPop", index });
    } catch (err) {
      setError(String(err));
    }
  };

  const dropStash = async (index: number) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendDestructiveIntent({ type: "stashDrop", index, confirmed: false });
    } catch (err) {
      setError(String(err));
    }
  };

  return {
    saveStash,
    saveStashIncludeUntracked,
    saveStashStaged,
    applyStash,
    popStash,
    dropStash,
  };
};
