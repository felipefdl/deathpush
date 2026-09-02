import { createMemo } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { sendIntent } from "../../lib/session-client";
import { Spinner } from "../ui/spinner";

export const ActionButton = () => {
  const status = useStore(repositoryStore, (s) => s.status);
  const actions = useStore(repositoryStore, (s) => s.actions);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const { setError, startOperation, endOperation } = repositoryStore.getState();

  const ahead = createMemo(() => status()?.ahead ?? 0);
  const behind = createMemo(() => status()?.behind ?? 0);
  const sync = createMemo(() => actions()?.sync);
  const busy = createMemo(() => operations().has("sync"));

  const handleSync = async () => {
    const current = sync();
    if (!current?.enabled) return;
    startOperation("sync");
    try {
      await sendIntent({ type: "sync" });
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("sync");
    }
  };

  return (
    <>
      {status() && sync()?.enabled ? (
        sync()?.kind === "fetch" ? (
          <button class="scm-toolbar-button" onClick={handleSync} disabled={busy()} title="Fetch">
            {busy() ? <Spinner /> : <span class="codicon codicon-cloud-download" />}
          </button>
        ) : (
          <button
            class="scm-toolbar-button"
            onClick={handleSync}
            disabled={busy()}
            title={`Sync: ${behind()}\u2193 ${ahead()}\u2191`}
          >
            {busy() ? <Spinner /> : <span class="codicon codicon-sync" />}
          </button>
        )
      ) : null}
    </>
  );
};
