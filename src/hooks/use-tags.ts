import { repositoryStore } from "../stores/repository-store";
import * as commands from "../lib/tauri-commands";

export const useTags = () => {
  const loadTags = async () => {
    const { setTags, setError } = repositoryStore.getState();
    try {
      const tags = await commands.listTags();
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  };

  const createTag = async (name: string, message?: string, commit?: string) => {
    const { setTags, setError } = repositoryStore.getState();
    try {
      const tags = await commands.createTag(name, message, commit);
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  };

  const removeTag = async (name: string) => {
    const { setTags, setError } = repositoryStore.getState();
    try {
      const tags = await commands.deleteTag(name);
      setTags(tags);
    } catch (err) {
      setError(String(err));
    }
  };

  const pushTagToRemote = async (name: string, remote: string = "origin") => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.pushTag(remote, name);
    } catch (err) {
      setError(String(err));
    }
  };

  const removeRemoteTag = async (name: string, remote: string = "origin") => {
    const { setError } = repositoryStore.getState();
    try {
      await commands.deleteRemoteTag(remote, name);
    } catch (err) {
      setError(String(err));
    }
  };

  return { loadTags, createTag, removeTag, pushTagToRemote, removeRemoteTag };
};
