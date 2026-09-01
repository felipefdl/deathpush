import { createMemo } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import * as commands from "../../lib/tauri-commands";
import { Spinner } from "../ui/spinner";

export const ActionButton = () => {
  const status = useStore(repositoryStore, (s) => s.status);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const { setError, startOperation, endOperation } = repositoryStore.getState();

  const branch = createMemo(() => status()?.headBranch);
  const ahead = createMemo(() => status()?.ahead ?? 0);
  const behind = createMemo(() => status()?.behind ?? 0);
  const isSyncing = createMemo(() => operations().has("push") || operations().has("pull"));
  const isFetching = createMemo(() => operations().has("fetch"));
  const busy = createMemo(() => isSyncing() || isFetching());

  const handleSync = async () => {
    if (!branch()) return;
    try {
      if (behind() > 0) {
        startOperation("pull");
        await commands.pull("origin", branch()!);
        endOperation("pull");
      }
      if (ahead() > 0) {
        startOperation("push");
        await commands.push("origin", branch()!);
        endOperation("push");
      }
    } catch (err) {
      endOperation("pull");
      endOperation("push");
      setError(String(err));
    }
  };

  const handleFetch = async () => {
    startOperation("fetch");
    try {
      await commands.fetchRemote("origin", true);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("fetch");
    }
  };

  return (
    <>
      {status() && branch() ? (
        ahead() > 0 || behind() > 0 ? (
          <button
            class="scm-toolbar-button"
            onClick={handleSync}
            disabled={busy()}
            title={`Sync: ${behind()}\u2193 ${ahead()}\u2191`}
          >
            {isSyncing() ? <Spinner /> : <span class="codicon codicon-sync" />}
          </button>
        ) : (
          <button class="scm-toolbar-button" onClick={handleFetch} disabled={busy()} title="Fetch">
            {isFetching() ? <Spinner /> : <span class="codicon codicon-cloud-download" />}
          </button>
        )
      ) : null}
    </>
  );
};
