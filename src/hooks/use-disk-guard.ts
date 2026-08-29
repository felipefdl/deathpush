import { useTauriEvent } from "./use-tauri-event";
import { explorerStore } from "../stores/explorer-store";
import { readFileContent } from "../lib/tauri-commands";
import { sha256Utf8 } from "../lib/pierre/sha";
import { watcherAction, type SaveSession } from "../lib/pierre/save-session";
import type { FileContent } from "../lib/git-types";

export type FileViewerDiskGuardInput = {
  selectedPath: string | null;
  session: SaveSession | null;
  readFileContent: (path: string) => Promise<FileContent>;
  sha256Utf8: (text: string) => Promise<string>;
  onReload: (content: FileContent, incomingSha: string) => void;
};

export const runFileViewerDiskGuard = async (input: FileViewerDiskGuardInput): Promise<void> => {
  const path = input.selectedPath;
  if (!path || !input.session) return;
  if (input.session.pendingSha !== null) return;

  const content = await input.readFileContent(path);
  const incomingSha = await input.sha256Utf8(content.content);
  if (watcherAction(input.session, incomingSha) === "reload") {
    input.onReload(content, incomingSha);
  }
};

export const useDiskGuard = (args: {
  getSession: () => SaveSession | null;
  onReload: (content: FileContent, incomingSha: string) => void;
}): void => {
  useTauriEvent("repository-changed", () => {
    void runFileViewerDiskGuard({
      selectedPath: explorerStore.getState().selectedPath,
      session: args.getSession(),
      readFileContent,
      sha256Utf8,
      onReload: args.onReload,
    }).catch(() => undefined);
  });
};
