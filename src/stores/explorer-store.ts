import { create } from "zustand";
import type { ExplorerEntry, FileContent } from "../lib/git-types";

interface ClipboardEntry {
  path: string;
  isDirectory: boolean;
  operation: "copy" | "cut";
}

interface CreatingIn {
  parentPath: string | null;
  type: "file" | "folder";
}

interface DragSource {
  path: string;
  isDirectory: boolean;
}

interface ExplorerState {
  selectedPath: string | null;
  fileContent: FileContent | null;
  fileFilter: string;
  expandedDirs: Set<string>;
  directoryCache: Map<string, ExplorerEntry[]>;
  clipboardEntry: ClipboardEntry | null;
  renamingPath: string | null;
  creatingIn: CreatingIn | null;
  dragSource: DragSource | null;
  dropTarget: string | null;
  isFileDirty: boolean;

  setSelectedPath: (path: string | null) => void;
  setFileContent: (content: FileContent | null) => void;
  setFileFilter: (filter: string) => void;
  toggleDir: (path: string) => void;
  expandDir: (path: string) => void;
  setDirectoryEntries: (path: string, entries: ExplorerEntry[]) => void;
  setClipboardEntry: (entry: ClipboardEntry | null) => void;
  setRenamingPath: (path: string | null) => void;
  setCreatingIn: (state: CreatingIn | null) => void;
  setDragSource: (source: DragSource | null) => void;
  setDropTarget: (target: string | null) => void;
  setIsFileDirty: (dirty: boolean) => void;
  clearCache: () => void;
}

export const useExplorerStore = create<ExplorerState>((set) => ({
  selectedPath: null,
  fileContent: null,
  fileFilter: "",
  expandedDirs: new Set<string>(),
  directoryCache: new Map<string, ExplorerEntry[]>(),
  clipboardEntry: null,
  renamingPath: null,
  creatingIn: null,
  dragSource: null,
  dropTarget: null,
  isFileDirty: false,

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

  expandDir: (path) =>
    set((state) => {
      if (state.expandedDirs.has(path)) return state;
      const next = new Set(state.expandedDirs);
      next.add(path);
      return { expandedDirs: next };
    }),

  setDirectoryEntries: (path, entries) =>
    set((state) => {
      const next = new Map(state.directoryCache);
      next.set(path, entries);
      return { directoryCache: next };
    }),

  setClipboardEntry: (clipboardEntry) => set({ clipboardEntry }),

  setRenamingPath: (renamingPath) => set({ renamingPath }),

  setCreatingIn: (creatingIn) => set({ creatingIn }),

  setDragSource: (dragSource) => set({ dragSource }),

  setDropTarget: (dropTarget) => set({ dropTarget }),

  setIsFileDirty: (isFileDirty) => set({ isFileDirty }),

  clearCache: () =>
    set({
      selectedPath: null,
      fileContent: null,
      fileFilter: "",
      expandedDirs: new Set<string>(),
      directoryCache: new Map<string, ExplorerEntry[]>(),
      clipboardEntry: null,
      renamingPath: null,
      creatingIn: null,
      dragSource: null,
      dropTarget: null,
      isFileDirty: false,
    }),
}));
