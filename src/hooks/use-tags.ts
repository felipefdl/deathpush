import { repositoryStore } from "../stores/repository-store";
import { sendDestructiveIntent, sendIntent } from "../lib/session-client";

export const useTags = () => {
  const createTag = async (name: string, message?: string, commit?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({
        type: "createTag",
        name,
        message: message ?? null,
        commit: commit ?? null,
      });
    } catch (err) {
      setError(String(err));
    }
  };

  const removeTag = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendDestructiveIntent({ type: "deleteTag", name, confirmed: false });
    } catch (err) {
      setError(String(err));
    }
  };

  const pushTagToRemote = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "pushTag", name });
    } catch (err) {
      setError(String(err));
    }
  };

  const removeRemoteTag = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "deleteRemoteTag", name });
    } catch (err) {
      setError(String(err));
    }
  };

  return { createTag, removeTag, pushTagToRemote, removeRemoteTag };
};
