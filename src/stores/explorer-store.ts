import { create } from "zustand";
import type { ExplorerEntry, FileContent } from "../lib/git-types";

interface ExplorerState {
  selectedPath: string | null;
  fileContent: FileContent | null;
  fileFilter: string;
  expandedDirs: Set<string>;
  directoryCache: Map<string, ExplorerEntry[]>;

  setSelectedPath: (path: string | null) => void;
  setFileContent: (content: FileContent | null) => void;
  setFileFilter: (filter: string) => void;
  toggleDir: (path: string) => void;
  setDirectoryEntries: (path: string, entries: ExplorerEntry[]) => void;
  clearCache: () => void;
}

export const useExplorerStore = create<ExplorerState>((set) => ({
  selectedPath: null,
  fileContent: null,
  fileFilter: "",
  expandedDirs: new Set<string>(),
  directoryCache: new Map<string, ExplorerEntry[]>(),

  setSelectedPath: (selectedPath) => set({ selectedPath }),

  setFileContent: (fileContent) => set({ fileContent }),

  setFileFilter: (fileFilter) => set({ fileFilter }),

  toggleDir: (path) =>
    set((state) => {
      const next = new Set(state.expandedDirs);
      if (next.has(path)) {
        next.delete(path);
      } else {
        next.add(path);
      }
      return { expandedDirs: next };
    }),

  setDirectoryEntries: (path, entries) =>
    set((state) => {
      const next = new Map(state.directoryCache);
      next.set(path, entries);
      return { directoryCache: next };
    }),

  clearCache: () =>
    set({
      selectedPath: null,
      fileContent: null,
      fileFilter: "",
      expandedDirs: new Set<string>(),
      directoryCache: new Map<string, ExplorerEntry[]>(),
    }),
}));
