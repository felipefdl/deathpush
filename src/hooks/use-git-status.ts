import { useRepository } from "./use-repository";
import { repositoryStore } from "../stores/repository-store";
import { useTauriEvent } from "./use-tauri-event";
import { sendIntent } from "../lib/session-client";
import { watcherAction, type SaveSession } from "../lib/pierre/save-session";
import type { DiffContent, PathsChanged } from "../lib/git-types";
import type { SelectedFile } from "../stores/repository-store";
import { getScmSession } from "../lib/pierre/scm-session-registry";
import { pathsChangedIntersects } from "./use-repository-events";

export const isScmWatcherTarget = (file: SelectedFile | null): boolean => file !== null && file.groupKind !== "merge";

export type ScmGuardDiff = DiffContent & { contentHash: string };

export type ScmDiskGuardInput = {
  selectedFile: SelectedFile | null;
  session: SaveSession | null;
  getFileDiff: (path: string, staged: boolean) => Promise<ScmGuardDiff>;
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
  const incomingSha = diff.contentHash;
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

  useTauriEvent<PathsChanged>("repository:paths-changed", (event) => {
    const { selectedFile } = repositoryStore.getState();
    if (!pathsChangedIntersects(event, selectedFile?.path ?? null)) return;
    const handle = getScmSession();
    void run({
      selectedFile,
      session: handle?.session ?? null,
      getFileDiff: async (path, staged) => {
        const result = await sendIntent({ type: "openScmDiff", path, staged });
        if (result.kind !== "diff") {
          throw new Error("Expected a diff payload");
        }
        return {
          path: result.payload.path,
          original: result.payload.original,
          modified: result.payload.modified,
          originalLanguage: result.payload.language,
          fileType: result.payload.fileType,
          contentHash: result.payload.contentHash,
        };
      },
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
  });

  return { refreshStatus };
};
