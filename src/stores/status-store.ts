import { createStore } from "zustand/vanilla";
import type {
  FileEntry,
  ResourceGroup,
  ResourceGroupKind,
  StatusEntry,
  StatusPatch,
  StatusPhase,
  StatusSnapshot,
} from "../lib/git-types";

export type ApplyStatusPatchResult = "applied" | "discarded" | "gap";

export const statusEntryKey = (group: ResourceGroupKind, path: string): string => `${group}\0${path}`;

export const groupsFromEntries = (entries: Iterable<StatusEntry>): ResourceGroup[] => {
  const merge: FileEntry[] = [];
  const index: FileEntry[] = [];
  const workingTree: FileEntry[] = [];

  for (const entry of entries) {
    const file: FileEntry = { path: entry.path, status: entry.status, renamePath: entry.renamePath };
    if (entry.group === "merge") {
      merge.push(file);
    } else if (entry.group === "index") {
      index.push(file);
    } else {
      workingTree.push(file);
    }
  }

  const groups: ResourceGroup[] = [];
  if (merge.length > 0) {
    groups.push({ kind: "merge", label: "Merge Changes", files: merge });
  }
  if (index.length > 0) {
    groups.push({ kind: "index", label: "Staged Changes", files: index });
  }
  if (workingTree.length > 0) {
    groups.push({ kind: "workingTree", label: "Changes", files: workingTree });
  }
  return groups;
};

type StatusState = {
  generation: number;
  revision: number;
  phase: StatusPhase;
  entries: Map<string, StatusEntry>;
  groups: ResourceGroup[];
};

const emptyState = (): StatusState => ({
  generation: 0,
  revision: 0,
  phase: "settled",
  entries: new Map(),
  groups: [],
});

export const statusStore = createStore<StatusState>(() => emptyState());

const commitEntries = (
  entries: Map<string, StatusEntry>,
  generation: number,
  revision: number,
  phase: StatusPhase
): void => {
  statusStore.setState({
    entries,
    groups: groupsFromEntries(entries.values()),
    generation,
    revision,
    phase,
  });
};

export const resetStatusStore = (): void => {
  statusStore.setState(emptyState());
};

export const applyStatusPatch = (patch: StatusPatch): ApplyStatusPatchResult => {
  const state = statusStore.getState();
  if (patch.generation < state.generation) {
    return "discarded";
  }
  if (patch.baseRevision !== state.revision) {
    return "gap";
  }

  const entries = new Map(state.entries);
  for (const removal of patch.removals) {
    entries.delete(statusEntryKey(removal.group, removal.path));
  }
  for (const upsert of patch.upserts) {
    entries.set(statusEntryKey(upsert.group, upsert.path), upsert);
  }
  commitEntries(entries, patch.generation, patch.revision, patch.phase);
  return "applied";
};

export const replaceFromSnapshot = (snapshot: StatusSnapshot): void => {
  const entries = new Map<string, StatusEntry>();
  for (const entry of snapshot.entries) {
    entries.set(statusEntryKey(entry.group, entry.path), entry);
  }
  commitEntries(entries, snapshot.generation, snapshot.revision, snapshot.phase);
};
