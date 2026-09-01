import { useTauriEvent } from "./use-tauri-event";
import { explorerStore } from "../stores/explorer-store";
import { readFileContent } from "../lib/tauri-commands";
import { sha256Utf8 } from "../lib/pierre/sha";
import { watcherAction, type SaveSession } from "../lib/pierre/save-session";
import type { FileContent, PathsChanged } from "../lib/git-types";
import { pathsChangedIntersects } from "./use-repository-events";
export type FileViewerDiskGuardInput = {
  selectedPath: string | null;
  session: SaveSession | null;
  readFileContent: (path: string) => Promise<FileContent>;
  sha256Utf8: (text: string) => Promise<string>;
  onReload: (content: FileContent, incomingSha: string) => void;
  isCurrent?: () => boolean;
};

export const runFileViewerDiskGuard = async (input: FileViewerDiskGuardInput): Promise<void> => {
  const path = input.selectedPath;
  if (!path || !input.session) return;
  if (input.session.pendingSha !== null) return;

  const content = await input.readFileContent(path);
  const incomingSha = await input.sha256Utf8(content.content);
  if (input.isCurrent && !input.isCurrent()) return;
  if (watcherAction(input.session, incomingSha) === "reload") {
    input.onReload(content, incomingSha);
  }
};

export const createFileViewerDiskGuard = (): ((input: FileViewerDiskGuardInput) => Promise<void>) => {
  let latest = 0;
  return async (input) => {
    const requestId = ++latest;
    await runFileViewerDiskGuard({
      ...input,
      isCurrent: () => requestId === latest && (input.isCurrent?.() ?? true),
    });
  };
};

export const useDiskGuard = (args: {
  getSession: () => SaveSession | null;
  onReload: (content: FileContent, incomingSha: string) => void;
}): void => {
  const run = createFileViewerDiskGuard();
  useTauriEvent<PathsChanged>("repository:paths-changed", (event) => {
    const selectedPath = explorerStore.getState().selectedPath;
    if (!pathsChangedIntersects(event, selectedPath)) return;
    const session = args.getSession();
    void run({
      selectedPath,
      session,
      readFileContent,
      sha256Utf8,
      onReload: args.onReload,
      isCurrent: () => explorerStore.getState().selectedPath === selectedPath && args.getSession() === session,
    }).catch(() => undefined);
  });
};
