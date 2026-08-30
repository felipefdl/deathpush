import { useRepository } from "./use-repository";
import { repositoryStore } from "../stores/repository-store";
import { useTauriEvent } from "./use-tauri-event";
import { throttle } from "../lib/throttle";
import { getFileDiff } from "../lib/tauri-commands";
import { sha256Utf8 } from "../lib/pierre/sha";
import { watcherAction, type SaveSession } from "../lib/pierre/save-session";
import type { DiffContent } from "../lib/git-types";
import type { SelectedFile } from "../stores/repository-store";
import { getScmSession } from "../components/pierre/pierre-file-diff";

export const isScmWatcherTarget = (file: SelectedFile | null): boolean => file !== null && file.groupKind !== "merge";

export type ScmDiskGuardInput = {
  selectedFile: SelectedFile | null;
  session: SaveSession | null;
  getFileDiff: (path: string, staged: boolean) => Promise<DiffContent>;
  sha256Utf8: (text: string) => Promise<string>;
  onReload: (diff: DiffContent, incomingSha: string) => void;
  isCurrent?: () => boolean;
};

export const runScmDiskGuard = async (input: ScmDiskGuardInput): Promise<void> => {
  const file = input.selectedFile;
  if (!file || !input.session) return;
  if (!isScmWatcherTarget(file)) return;
  if (input.session.path !== file.path) return;
  const ignorePendingSha = file.groupKind === "index";
  if (!ignorePendingSha && input.session.pendingSha !== null) return;

  const diff = await input.getFileDiff(file.path, file.staged);
  const incomingSha = await input.sha256Utf8(diff.modified);
  if (input.isCurrent && !input.isCurrent()) return;
  const session = ignorePendingSha ? { ...input.session, pendingSha: null } : input.session;
  if (watcherAction(session, incomingSha) === "reload") {
    input.onReload(diff, incomingSha);
  }
};

export const createScmDiskGuard = (): ((input: ScmDiskGuardInput) => Promise<void>) => {
  let latest = 0;
  return async (input) => {
    const requestId = ++latest;
    await runScmDiskGuard({
      ...input,
      isCurrent: () => requestId === latest && (input.isCurrent?.() ?? true),
    });
  };
};

export const useGitStatus = () => {
  const { refreshStatus } = useRepository();
  const run = createScmDiskGuard();

  const handleChange = throttle(() => {
    void refreshStatus();
    const { selectedFile } = repositoryStore.getState();
    const handle = getScmSession();
    void run({
      selectedFile,
      session: handle?.session ?? null,
      getFileDiff,
      sha256Utf8,
      onReload: (diff, incomingSha) => {
        handle?.reload(diff, incomingSha);
      },
      isCurrent: () => {
        const current = repositoryStore.getState().selectedFile;
        return (
          current?.path === selectedFile?.path &&
          current?.staged === selectedFile?.staged &&
          current?.groupKind === selectedFile?.groupKind &&
          getScmSession() === handle
        );
      },
    }).catch(() => undefined);
  }, 1000);

  useTauriEvent("repository-changed", handleChange);

  return { refreshStatus };
};
