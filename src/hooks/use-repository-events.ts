import { useTauriEvent } from "./use-tauri-event";
import {
  applyStatusPatch,
  applyStatusPatches,
  replaceFromSnapshot,
  resetStatusStore,
  statusSession,
  statusStore,
  type ApplyStatusPatchResult,
} from "../stores/status-store";
import { repositoryStore } from "../stores/repository-store";
import { getStatusSnapshot } from "../lib/tauri-commands";
import type { PathsChanged, RepositoryMetadata, StatusPatch, StatusSnapshot } from "../lib/git-types";

const STORM_FLUSH_MS = 500;

type QueuedPatch = {
  session: number;
  patch: StatusPatch;
};

let pendingPatches: QueuedPatch[] = [];
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

const cancelScheduledFlush = (): void => {
  if (raf) {
    cancelAnimationFrame(raf);
    raf = 0;
  }
  if (stormTimer) {
    clearTimeout(stormTimer);
    stormTimer = null;
  }
};

export const beginRepositorySession = (): void => {
  resetStatusStore();
  pendingPatches = [];
  cancelScheduledFlush();
};

export const enqueueStatusPatch = (patch: StatusPatch): void => {
  pendingPatches.push({ session: statusSession(), patch });
};

const publishStatusProjection = (metadata?: RepositoryMetadata): void => {
  if (metadata) {
    repositoryStore.getState().applyMetadata(metadata);
  }
  repositoryStore.getState().syncStatusGroups();
};

export const applyRecoveredSnapshot = (snapshot: StatusSnapshot, session: number, root: string | null): boolean => {
  if (statusSession() !== session) return false;
  if ((repositoryStore.getState().status?.root ?? null) !== root) return false;
  if (snapshot.metadata.root !== root) return false;
  if (!replaceFromSnapshot(snapshot)) return false;
  publishStatusProjection(snapshot.metadata);
  return true;
};

export const applyIncomingPatch = async (
  patch: StatusPatch,
  recover: () => Promise<StatusSnapshot>
): Promise<ApplyStatusPatchResult> => {
  const result = applyStatusPatch(patch);
  if (result === "discarded") {
    return result;
  }
  if (result === "gap") {
    const session = statusSession();
    const root = repositoryStore.getState().status?.root ?? null;
    const snapshot = await recover();
    applyRecoveredSnapshot(snapshot, session, root);
    return result;
  }
  publishStatusProjection(patch.metadata);
  return result;
};

export const flushPendingPatches = (): Promise<void> => {
  cancelScheduledFlush();
  if (flushing) return Promise.resolve();
  const queued = pendingPatches;
  pendingPatches = [];
  if (queued.length === 0) return Promise.resolve();
  flushing = true;
  const currentSession = statusSession();
  const currentRoot = repositoryStore.getState().status?.root ?? null;
  return (async () => {
    try {
      const patches: StatusPatch[] = [];
      let lastMetadata: RepositoryMetadata | undefined;
      for (const item of queued) {
        if (item.session !== currentSession || item.session !== statusSession()) continue;
        patches.push(item.patch);
        if (item.patch.metadata) lastMetadata = item.patch.metadata;
      }
      if (patches.length === 0) return;
      const result = applyStatusPatches(patches);
      if (result === "discarded") return;
      if (result === "gap") {
        const snapshot = await getStatusSnapshot();
        applyRecoveredSnapshot(snapshot, currentSession, currentRoot);
        return;
      }
      if (statusSession() !== currentSession) return;
      if ((repositoryStore.getState().status?.root ?? null) !== currentRoot) return;
      publishStatusProjection(lastMetadata);
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
      stormTimer = setTimeout(() => {
        void flushPendingPatches();
      }, STORM_FLUSH_MS);
    }
    return;
  }
  if (!raf) {
    raf = requestAnimationFrame(() => {
      void flushPendingPatches();
    });
  }
};

export const useRepositoryEvents = (): void => {
  useTauriEvent<StatusPatch>("repository:status-patch", (payload) => {
    enqueueStatusPatch(payload);
    schedulePatchFlush(payload.phase === "storm" || statusStore.getState().phase === "storm");
  });
};
