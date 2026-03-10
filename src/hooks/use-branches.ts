import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useBranches = () => {
  const { setBranches, setStatus, setError } = useRepositoryStore();

  const loadBranches = useCallback(async () => {
    try {
      const branches = await commands.listBranches();
      setBranches(branches);
    } catch (err) {
      setError(String(err));
    }
  }, [setBranches, setError]);

  const switchBranch = useCallback(async (name: string) => {
    try {
      const status = await commands.checkoutBranch(name);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const createNewBranch = useCallback(async (name: string, from?: string) => {
    try {
      const status = await commands.createBranch(name, from);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const removeBranch = useCallback(async (name: string, force: boolean = false) => {
    try {
      await commands.deleteBranch(name, force);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  }, [loadBranches, setError]);

  const renameBranch = useCallback(async (oldName: string, newName: string) => {
    try {
      const status = await commands.renameBranch(oldName, newName);
      setStatus(status);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError, loadBranches]);

  const mergeBranch = useCallback(async (name: string) => {
    try {
      const status = await commands.mergeBranch(name);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const rebaseBranch = useCallback(async (name: string) => {
    try {
      const status = await commands.rebaseBranch(name);
      setStatus(status);
    } catch (err) {
      setError(String(err));
    }
  }, [setStatus, setError]);

  const removeRemoteBranch = useCallback(async (remote: string, name: string) => {
    try {
      await commands.deleteRemoteBranch(remote, name);
      await loadBranches();
    } catch (err) {
      setError(String(err));
    }
  }, [loadBranches, setError]);

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
