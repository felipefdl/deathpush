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

type IndexedGroupKind = "merge" | "index" | "workingTree";

type GroupFiles = {
  merge: FileEntry[];
  index: FileEntry[];
  workingTree: FileEntry[];
};

const GROUP_LABEL: Record<IndexedGroupKind, string> = {
  merge: "Merge Changes",
  index: "Staged Changes",
  workingTree: "Changes",
};

const INDEXED_GROUPS: IndexedGroupKind[] = ["merge", "index", "workingTree"];

export const statusEntryKey = (group: ResourceGroupKind, path: string): string => `${group}\0${path}`;

const indexedGroup = (group: ResourceGroupKind): IndexedGroupKind =>
  group === "merge" || group === "index" ? group : "workingTree";

const toFileEntry = (entry: StatusEntry): FileEntry => ({
  path: entry.path,
  status: entry.status,
  renamePath: entry.renamePath,
});

const emptyGroupFiles = (): GroupFiles => ({
  merge: [],
  index: [],
  workingTree: [],
});

const groupsFromGroupFiles = (groupFiles: GroupFiles): ResourceGroup[] => {
  const groups: ResourceGroup[] = [];
  for (const kind of INDEXED_GROUPS) {
    const files = groupFiles[kind];
    if (files.length > 0) {
      groups.push({ kind, label: GROUP_LABEL[kind], files });
    }
  }
  return groups;
};

type StatusState = {
  generation: number;
  revision: number;
  phase: StatusPhase;
  entries: Map<string, StatusEntry>;
  groupFiles: GroupFiles;
  groups: ResourceGroup[];
};

const emptyState = (): StatusState => ({
  generation: 0,
  revision: 0,
  phase: "settled",
  entries: new Map(),
  groupFiles: emptyGroupFiles(),
  groups: [],
});

export const statusStore = createStore<StatusState>(() => emptyState());

let session = 0;

export const statusSession = (): number => session;

const commitState = (
  entries: Map<string, StatusEntry>,
  groupFiles: GroupFiles,
  generation: number,
  revision: number,
  phase: StatusPhase
): void => {
  statusStore.setState({
    entries,
    groupFiles,
    groups: groupsFromGroupFiles(groupFiles),
    generation,
    revision,
    phase,
  });
};

export const resetStatusStore = (): void => {
  session += 1;
  statusStore.setState(emptyState());
};

export const applyStatusPatches = (patches: StatusPatch[]): ApplyStatusPatchResult => {
  if (patches.length === 0) return "applied";

  const state = statusStore.getState();
  const entries = new Map(state.entries);
  const groupFiles: GroupFiles = {
    merge: state.groupFiles.merge,
    index: state.groupFiles.index,
    workingTree: state.groupFiles.workingTree,
  };

  let generation = state.generation;
  let revision = state.revision;
  let phase = state.phase;
  let applied = false;
  const dirty = new Set<IndexedGroupKind>();

  for (const patch of patches) {
    if (patch.generation < generation) continue;
    if (patch.baseRevision !== revision) {
      return "gap";
    }

    for (const removal of patch.removals) {
      const key = statusEntryKey(removal.group, removal.path);
      if (!entries.delete(key)) continue;
      dirty.add(indexedGroup(removal.group));
    }
    for (const upsert of patch.upserts) {
      const key = statusEntryKey(upsert.group, upsert.path);
      const previous = entries.get(key);
      entries.set(key, upsert);
      const kind = indexedGroup(upsert.group);
      if (previous && indexedGroup(previous.group) !== kind) {
        dirty.add(indexedGroup(previous.group));
      }
      dirty.add(kind);
    }

    generation = patch.generation;
    revision = patch.revision;
    phase = patch.phase;
    applied = true;
  }

  if (!applied) return "discarded";

  for (const kind of dirty) {
    const files: FileEntry[] = [];
    for (const entry of entries.values()) {
      if (indexedGroup(entry.group) === kind) {
        files.push(toFileEntry(entry));
      }
    }
    groupFiles[kind] = files;
  }

  commitState(entries, groupFiles, generation, revision, phase);
  return "applied";
};

export const applyStatusPatch = (patch: StatusPatch): ApplyStatusPatchResult => applyStatusPatches([patch]);

export const replaceFromSnapshot = (snapshot: StatusSnapshot): boolean => {
  const state = statusStore.getState();
  if (snapshot.generation < state.generation) return false;
  if (snapshot.generation === state.generation && snapshot.revision < state.revision) return false;

  const entries = new Map<string, StatusEntry>();
  const groupFiles = emptyGroupFiles();
  for (const entry of snapshot.entries) {
    entries.set(statusEntryKey(entry.group, entry.path), entry);
    groupFiles[indexedGroup(entry.group)].push(toFileEntry(entry));
  }
  commitState(entries, groupFiles, snapshot.generation, snapshot.revision, snapshot.phase);
  return true;
};
