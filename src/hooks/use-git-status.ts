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

export const isScmWatcherTarget = (file: SelectedFile | null): boolean =>
  file !== null && file.groupKind !== "index" && file.groupKind !== "merge";

export const runScmDiskGuard = async (input: {
  selectedFile: SelectedFile | null;
  session: SaveSession | null;
  getFileDiff: (path: string, staged: boolean) => Promise<DiffContent>;
  sha256Utf8: (text: string) => Promise<string>;
  onReload: (diff: DiffContent, incomingSha: string) => void;
}): Promise<void> => {
  const file = input.selectedFile;
  if (!file || !input.session) return;
  if (!isScmWatcherTarget(file)) return;
  if (input.session.path !== file.path) return;
  if (input.session.pendingSha !== null) return;

  const diff = await input.getFileDiff(file.path, file.staged);
  const incomingSha = await input.sha256Utf8(diff.modified);
  if (watcherAction(input.session, incomingSha) === "reload") {
    input.onReload(diff, incomingSha);
  }
};

export const useGitStatus = () => {
  const { refreshStatus } = useRepository();

  const handleChange = throttle(() => {
    void refreshStatus();
    const { selectedFile } = repositoryStore.getState();
    const handle = getScmSession();
    void runScmDiskGuard({
      selectedFile,
      session: handle?.session ?? null,
      getFileDiff,
      sha256Utf8,
      onReload: (diff, incomingSha) => {
        handle?.reload(diff, incomingSha);
      },
    }).catch(() => undefined);
  }, 1000);

  useTauriEvent("repository-changed", handleChange);

  return { refreshStatus };
};
