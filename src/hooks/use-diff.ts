import type { ResourceGroupKind } from "../lib/git-types";
import { repositoryStore } from "../stores/repository-store";
import { acceptedDiff, sendIntent } from "../lib/session-client";
import { clearScmDiffPayload, rememberScmDiffPayload } from "../lib/pierre/scm-diff-payload";

const isDiffEqual = (
  a: { path: string; original: string; modified: string; fileType: string } | null,
  b: { path: string; original: string; modified: string; fileType: string } | null
) => {
  if (a === b) return true;
  if (!a || !b) return false;
  return a.path === b.path && a.original === b.original && a.modified === b.modified && a.fileType === b.fileType;
};

const inflight = new Map<string, Promise<void>>();

export const useDiff = () => {
  const loadDiff = (path: string, staged: boolean, groupKind: ResourceGroupKind = "workingTree"): Promise<void> => {
    const key = `${groupKind}:${staged ? "1" : "0"}:${path}`;
    const existing = inflight.get(key);
    if (existing) {
      const selected = repositoryStore.getState().selectedFile;
      if (selected && selected.path === path && selected.staged === staged && selected.groupKind === groupKind) {
        return existing;
      }
    }

    const run = (async () => {
      const { setDiff, setSelectedFile, setError } = repositoryStore.getState();
      setSelectedFile({ path, staged, groupKind });
      const requested = repositoryStore.getState();
      const loadId = requested.selectedLoadId;
      const requestGeneration = requested.sessionGeneration;
      const requestRoot = requested.status?.root;
      try {
        const result = await sendIntent({ type: "openScmDiff", path, staged, groupKind });
        const current = repositoryStore.getState();
        if (current.selectedLoadId !== loadId) return;
        if (current.sessionGeneration !== requestGeneration) return;
        if (requestRoot !== undefined && current.status?.root !== requestRoot) return;
        if (!acceptedDiff(result)) return;
        const diff = {
          path: result.payload.path,
          original: result.payload.original,
          modified: result.payload.modified,
          originalLanguage: result.payload.language,
          fileType: result.payload.fileType,
        };
        if (!isDiffEqual(current.diff, diff)) {
          rememberScmDiffPayload(
            { path: result.payload.path, staged: result.payload.staged, groupKind, loadId },
            result.payload
          );
          setDiff(diff);
        } else {
          current.bindDiffToCurrentLoad();
        }
      } catch (err) {
        if (repositoryStore.getState().selectedLoadId !== loadId) return;
        setError(String(err));
        setDiff(null);
      }
    })();

    inflight.set(key, run);
    void run.finally(() => {
      if (inflight.get(key) === run) inflight.delete(key);
    });
    return run;
  };

  const clearDiff = () => {
    const { setDiff, setSelectedFile } = repositoryStore.getState();
    setSelectedFile(null);
    setDiff(null);
    clearScmDiffPayload();
    void sendIntent({ type: "clearFile" });
  };

  return { loadDiff, clearDiff };
};
