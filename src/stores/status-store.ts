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

const cloneGroup = (groupFiles: GroupFiles, kind: IndexedGroupKind, cloned: Set<IndexedGroupKind>): FileEntry[] => {
  if (!cloned.has(kind)) {
    groupFiles[kind] = groupFiles[kind].slice();
    cloned.add(kind);
  }
  return groupFiles[kind];
};

const removeFromGroup = (files: FileEntry[], index: Map<string, number>, path: string): void => {
  const at = index.get(path);
  if (at === undefined) return;
  files.splice(at, 1);
  index.delete(path);
  for (let position = at; position < files.length; position += 1) {
    const file = files[position];
    if (file) index.set(file.path, position);
  }
};

const upsertInGroup = (files: FileEntry[], index: Map<string, number>, file: FileEntry): void => {
  const at = index.get(file.path);
  if (at !== undefined) {
    files[at] = file;
    return;
  }
  index.set(file.path, files.length);
  files.push(file);
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
  const cloned = new Set<IndexedGroupKind>();
  const indexes = new Map<IndexedGroupKind, Map<string, number>>();

  const indexOf = (kind: IndexedGroupKind): Map<string, number> => {
    let index = indexes.get(kind);
    if (!index) {
      const files = cloneGroup(groupFiles, kind, cloned);
      index = new Map(files.map((file, position) => [file.path, position]));
      indexes.set(kind, index);
    }
    return index;
  };

  let generation = state.generation;
  let revision = state.revision;
  let phase = state.phase;
  let applied = false;

  for (const patch of patches) {
    if (patch.generation < generation) continue;
    if (patch.baseRevision !== revision) {
      return "gap";
    }

    for (const removal of patch.removals) {
      const key = statusEntryKey(removal.group, removal.path);
      if (!entries.delete(key)) continue;
      const kind = indexedGroup(removal.group);
      removeFromGroup(cloneGroup(groupFiles, kind, cloned), indexOf(kind), removal.path);
    }
    for (const upsert of patch.upserts) {
      const key = statusEntryKey(upsert.group, upsert.path);
      const previous = entries.get(key);
      entries.set(key, upsert);
      const kind = indexedGroup(upsert.group);
      if (previous && indexedGroup(previous.group) !== kind) {
        const previousKind = indexedGroup(previous.group);
        removeFromGroup(cloneGroup(groupFiles, previousKind, cloned), indexOf(previousKind), previous.path);
      }
      upsertInGroup(cloneGroup(groupFiles, kind, cloned), indexOf(kind), toFileEntry(upsert));
    }

    generation = patch.generation;
    revision = patch.revision;
    phase = patch.phase;
    applied = true;
  }

  if (!applied) return "discarded";
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
