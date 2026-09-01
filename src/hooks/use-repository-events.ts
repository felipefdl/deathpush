import { useTauriEvent } from "./use-tauri-event";
import { applyStatusPatch, replaceFromSnapshot, statusStore } from "../stores/status-store";
import { repositoryStore } from "../stores/repository-store";
import { getStatusSnapshot } from "../lib/tauri-commands";
import type { ApplyStatusPatchResult } from "../stores/status-store";
import type { PathsChanged, StatusPatch, StatusSnapshot } from "../lib/git-types";

const STORM_FLUSH_MS = 500;

let pendingPatches: StatusPatch[] = [];
let raf = 0;
let stormTimer: ReturnType<typeof setTimeout> | null = null;
let flushing = false;

export const pathsChangedIntersects = (event: PathsChanged, target: string | null): boolean => {
  if (!target) return false;
  if (event.scope === "repository") return true;
  return event.paths.some((path) => {
    if (event.scope === "exact") return path === target;
    return target === path || target.startsWith(`${path}/`) || path.startsWith(`${target}/`);
  });
};

export const shouldRefreshExplorer = (event: PathsChanged): boolean =>
  event.scope === "repository" || event.scope === "subtree" || event.kind === "structural";

export const applyIncomingPatch = async (
  patch: StatusPatch,
  recover: () => Promise<StatusSnapshot>
): Promise<ApplyStatusPatchResult> => {
  const result = applyStatusPatch(patch);
  if (result === "discarded") {
    return result;
  }
  if (result === "gap") {
    const snapshot = await recover();
    replaceFromSnapshot(snapshot);
    repositoryStore.getState().applyMetadata(snapshot.metadata);
    repositoryStore.getState().syncStatusGroups();
    return result;
  }
  if (patch.metadata) {
    repositoryStore.getState().applyMetadata(patch.metadata);
  }
  repositoryStore.getState().syncStatusGroups();
  return result;
};

const flushPendingPatches = (): void => {
  raf = 0;
  if (stormTimer) {
    clearTimeout(stormTimer);
    stormTimer = null;
  }
  if (flushing) return;
  const patches = pendingPatches;
  pendingPatches = [];
  if (patches.length === 0) return;
  flushing = true;
  void (async () => {
    try {
      for (const patch of patches) {
        await applyIncomingPatch(patch, getStatusSnapshot);
      }
    } finally {
      flushing = false;
      if (pendingPatches.length > 0) {
        schedulePatchFlush(statusStore.getState().phase === "storm");
      }
    }
  })();
};

const schedulePatchFlush = (storm: boolean): void => {
  if (storm) {
    if (raf) {
      cancelAnimationFrame(raf);
      raf = 0;
    }
    if (!stormTimer) {
      stormTimer = setTimeout(flushPendingPatches, STORM_FLUSH_MS);
    }
    return;
  }
  if (!raf) {
    raf = requestAnimationFrame(flushPendingPatches);
  }
};

export const useRepositoryEvents = (): void => {
  useTauriEvent<StatusPatch>("repository:status-patch", (payload) => {
    pendingPatches.push(payload);
    schedulePatchFlush(payload.phase === "storm" || statusStore.getState().phase === "storm");
  });
};
