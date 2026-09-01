import { repositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useBranches = () => {
  const loadBranches = async () => {
    const { setBranches, setError } = repositoryStore.getState();
    try {
      const branches = await commands.listBranches();
      setBranches(branches);
    } catch (err) {
      setError(String(err));
    }
  };

  const switchBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.checkoutBranch(name);
    } catch (err) {
      setError(String(err));
    }
  };

  const createNewBranch = async (name: string, from?: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.createBranch(name, from);
    } catch (err) {
      setError(String(err));
    }
  };

  const removeBranch = async (name: string, force: boolean = false) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.deleteBranch(name, force);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  };

  const renameBranch = async (oldName: string, newName: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.renameBranch(oldName, newName);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  };

  const mergeBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.mergeBranch(name);
    } catch (err) {
      setError(String(err));
    }
  };

  const rebaseBranch = async (name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.rebaseBranch(name);
    } catch (err) {
      setError(String(err));
    }
  };

  const removeRemoteBranch = async (remote: string, name: string) => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.deleteRemoteBranch(remote, name);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  };

  return {
    loadBranches,
    switchBranch,
    createNewBranch,
    removeBranch,
    renameBranch,
    mergeBranch,
    rebaseBranch,
    removeRemoteBranch,
  };
};
