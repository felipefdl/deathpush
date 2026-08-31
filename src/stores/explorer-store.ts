import { createStore } from "zustand/vanilla";
import type { FileContent } from "../lib/git-types";

interface ClipboardEntry {
  path: string;
  isDirectory: boolean;
  operation: "copy" | "cut";
}

interface SelectedTreeEntry {
  path: string;
  isDirectory: boolean;
}

interface ExplorerState {
  selectedPath: string | null;
  selectedTreeEntry: SelectedTreeEntry | null;
  fileContent: FileContent | null;
  fileFilter: string;
  clipboardEntry: ClipboardEntry | null;
  isFileDirty: boolean;
  revealLine: number | null;
  treeExpandedPaths: string[];

  setSelectedPath: (path: string | null) => void;
  setSelectedTreeEntry: (entry: SelectedTreeEntry | null) => void;
  setFileContent: (content: FileContent | null) => void;
  setRevealLine: (line: number | null) => void;
  setFileFilter: (filter: string) => void;
  setClipboardEntry: (entry: ClipboardEntry | null) => void;
  setIsFileDirty: (dirty: boolean) => void;
  setTreeExpandedPaths: (paths: string[]) => void;
  reset: () => void;
}

export const explorerStore = createStore<ExplorerState>((set) => ({
  selectedPath: null,
  selectedTreeEntry: null,
  fileContent: null,
  fileFilter: "",
  clipboardEntry: null,
  isFileDirty: false,
  revealLine: null,
  treeExpandedPaths: [],

  setSelectedPath: (selectedPath) => set({ selectedPath }),
  setSelectedTreeEntry: (selectedTreeEntry) => set({ selectedTreeEntry }),
  setFileContent: (fileContent) => set({ fileContent }),
  setRevealLine: (revealLine) => set({ revealLine }),
  setFileFilter: (fileFilter) => set({ fileFilter }),
  setClipboardEntry: (clipboardEntry) => set({ clipboardEntry }),
  setIsFileDirty: (isFileDirty) => set({ isFileDirty }),
  setTreeExpandedPaths: (treeExpandedPaths) => set({ treeExpandedPaths }),
  reset: () =>
    set({
      selectedPath: null,
      selectedTreeEntry: null,
      fileContent: null,
      fileFilter: "",
      clipboardEntry: null,
      isFileDirty: false,
      revealLine: null,
      treeExpandedPaths: [],
    }),
}));
