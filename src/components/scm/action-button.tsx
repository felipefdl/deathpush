import { useCallback } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";
import { Spinner } from "../ui/spinner";

export const ActionButton = () => {
  const { status, setStatus, setError, startOperation, endOperation, operations } = useRepositoryStore();

  const branch = status?.headBranch;
  const ahead = status?.ahead ?? 0;
  const behind = status?.behind ?? 0;

  const isSyncing = operations.has("push") || operations.has("pull");
  const isFetching = operations.has("fetch");
  const busy = isSyncing || isFetching;

  const handleSync = useCallback(async () => {
    if (!branch) return;
    let newStatus;
    try {
      if (behind > 0) {
        startOperation("pull");
        newStatus = await commands.pull("origin", branch);
        endOperation("pull");
      }
      if (ahead > 0) {
        startOperation("push");
        newStatus = await commands.push("origin", branch);
        endOperation("push");
      }
      if (newStatus) setStatus(newStatus);
    } catch (err) {
      endOperation("pull");
      endOperation("push");
      setError(String(err));
    }
  }, [branch, ahead, behind, setStatus, setError, startOperation, endOperation]);

  const handleFetch = useCallback(async () => {
    startOperation("fetch");
    try {
      const newStatus = await commands.fetchRemote("origin", true);
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("fetch");
    }
  }, [setStatus, setError, startOperation, endOperation]);

  if (!status || !branch) return null;

  if (ahead > 0 || behind > 0) {
    return (
      <button className="scm-toolbar-button" onClick={handleSync} disabled={busy} title={`Sync: ${behind}\u2193 ${ahead}\u2191`}>
        {isSyncing ? <Spinner /> : <span className="codicon codicon-sync" />}
      </button>
    );
  }

  return (
    <button className="scm-toolbar-button" onClick={handleFetch} disabled={busy} title="Fetch">
      {isFetching ? <Spinner /> : <span className="codicon codicon-cloud-download" />}
    </button>
  );
};
