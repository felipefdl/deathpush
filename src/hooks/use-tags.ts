import { useCallback } from "react";
import { useRepositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useTags = () => {
  const { setTags, setError } = useRepositoryStore();

  const loadTags = useCallback(async () => {
    try {
      const tags = await commands.listTags();
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  }, [setTags, setError]);

  const createTag = useCallback(async (name: string, message?: string, commit?: string) => {
    try {
      const tags = await commands.createTag(name, message, commit);
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  }, [setTags, setError]);

  const removeTag = useCallback(async (name: string) => {
    try {
      const tags = await commands.deleteTag(name);
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  }, [setTags, setError]);

  const pushTagToRemote = useCallback(async (name: string, remote: string = "origin") => {
    try {
      await commands.pushTag(remote, name);
    } catch (err) {
      setError(String(err));
    }
  }, [setError]);

  const removeRemoteTag = useCallback(async (name: string, remote: string = "origin") => {
    try {
      await commands.deleteRemoteTag(remote, name);
    } catch (err) {
      setError(String(err));
    }
  }, [setError]);

  return { loadTags, createTag, removeTag, pushTagToRemote, removeRemoteTag };
};
