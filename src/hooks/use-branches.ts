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

  return { loadBranches, switchBranch, createNewBranch, removeBranch };
};
