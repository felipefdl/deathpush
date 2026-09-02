import { repositoryStore } from "../stores/repository-store";
import { sendDestructiveIntent, sendIntent } from "../lib/session-client";

export const useBranches = () => {
  const switchBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "checkoutBranch", name });
    } catch (err) {
      setError(String(err));
    }
  };

  const createNewBranch = async (name: string, from?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "createBranch", name, from: from ?? null });
    } catch (err) {
      setError(String(err));
    }
  };

  const removeBranch = async (name: string, force: boolean = false) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendDestructiveIntent({ type: "deleteBranch", name, force, confirmed: false });
    } catch (err) {
      setError(String(err));
    }
  };

  const renameBranch = async (oldName: string, newName: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "renameBranch", oldName, newName });
    } catch (err) {
      setError(String(err));
    }
  };

  const mergeBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "mergeBranch", name });
    } catch (err) {
      setError(String(err));
    }
  };

  const rebaseBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "rebaseBranch", name });
    } catch (err) {
      setError(String(err));
    }
  };

  const removeRemoteBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await sendIntent({ type: "deleteRemoteBranch", name });
    } catch (err) {
      setError(String(err));
    }
  };

  return {
    switchBranch,
    createNewBranch,
    removeBranch,
    renameBranch,
    mergeBranch,
    rebaseBranch,
    removeRemoteBranch,
  };
};
