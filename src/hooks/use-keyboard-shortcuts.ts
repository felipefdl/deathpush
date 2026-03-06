import { useEffect } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../stores/repository-store";
import { useLayoutStore } from "../stores/layout-store";
import { toggleTerminal } from "../lib/toggle-terminal";
import { buildFlatFileList } from "../lib/flat-file-list";
import * as commands from "../lib/tauri-commands";

export const useKeyboardShortcuts = () => {
  const {
    setStatus, setError,
    setFocusedIndex, setSelectedFile, setDiff,
    clearFileSelection, startOperation, endOperation,
  } = useRepositoryStore();
  const { diffMode, setDiffMode, mainView, setMainView } = useLayoutStore();

  useEffect(() => {
    let chordK = false;
    let chordTimer: ReturnType<typeof setTimeout> | null = null;

    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;

      // Chord: Cmd+K Cmd+T -> open theme picker
      if (chordK && isMod && e.key === "t") {
        e.preventDefault();
        chordK = false;
        if (chordTimer) clearTimeout(chordTimer);
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
        return;
      }

      // Chord: Cmd+K Cmd+I -> open icon theme picker
      if (chordK && isMod && e.key === "i") {
        e.preventDefault();
        chordK = false;
        if (chordTimer) clearTimeout(chordTimer);
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
        return;
      }

      if (isMod && e.key === "k") {
        e.preventDefault();
        chordK = true;
        if (chordTimer) clearTimeout(chordTimer);
        chordTimer = setTimeout(() => { chordK = false; }, 1500);
        return;
      }

      chordK = false;

      if (isMod && e.key === "s") {
        e.preventDefault();
        return;
      }

      if (isMod && e.key === ",") {
        e.preventDefault();
        const layout = useLayoutStore.getState();
        layout.setMainView(layout.mainView === "settings" ? "changes" : "settings");
        return;
      }

      if (isMod && e.key === "`") {
        e.preventDefault();
        toggleTerminal();
        return;
      }

      const target = e.target as HTMLElement;
      const isInput = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;

      // Ctrl/Cmd+Shift+G: Focus SCM (refresh status)
      if (isMod && e.shiftKey && e.key === "G") {
        e.preventDefault();
        commands.getStatus().then(setStatus).catch((err) => setError(String(err)));
        return;
      }

      // Ctrl/Cmd+Shift+P: Toggle diff mode
      if (isMod && e.shiftKey && e.key === "P") {
        e.preventDefault();
        setDiffMode(diffMode === "inline" ? "sideBySide" : "inline");
        return;
      }

      // Skip navigation keys when focus is in an input
      if (isInput) return;

      const state = useRepositoryStore.getState();
      const { status, fileFilter, focusedIndex } = state;
      if (!status) return;

      const flatList = buildFlatFileList(status.groups, fileFilter);
      if (flatList.length === 0) return;

      // Arrow Down: move focus down
      if (e.key === "ArrowDown") {
        e.preventDefault();
        const next = focusedIndex === null ? 0 : Math.min(focusedIndex + 1, flatList.length - 1);
        setFocusedIndex(next);
        return;
      }

      // Arrow Up: move focus up
      if (e.key === "ArrowUp") {
        e.preventDefault();
        const next = focusedIndex === null ? flatList.length - 1 : Math.max(focusedIndex - 1, 0);
        setFocusedIndex(next);
        return;
      }

      // Escape: clear focus and selection
      if (e.key === "Escape") {
        e.preventDefault();
        setFocusedIndex(null);
        setSelectedFile(null);
        setDiff(null);
        clearFileSelection();
        return;
      }

      // Following keys require a focused item
      if (focusedIndex === null || focusedIndex >= flatList.length) return;
      const focused = flatList[focusedIndex];
      const isStaged = focused.groupKind === "index";

      // Enter: open diff
      if (e.key === "Enter") {
        e.preventDefault();
        setSelectedFile({ path: focused.path, staged: isStaged });
        commands.getFileDiff(focused.path, isStaged)
          .then((diff) => {
            useRepositoryStore.getState().setDiff(diff);
          })
          .catch((err) => setError(String(err)));
        return;
      }

      // Space: toggle stage/unstage
      if (e.key === " ") {
        e.preventDefault();
        if (isStaged) {
          startOperation("unstage");
          commands.unstageFiles([focused.path])
            .then(setStatus)
            .catch((err) => setError(String(err)))
            .finally(() => endOperation("unstage"));
        } else {
          startOperation("stage");
          commands.stageFiles([focused.path])
            .then(setStatus)
            .catch((err) => setError(String(err)))
            .finally(() => endOperation("stage"));
        }
        return;
      }

      // Delete/Backspace: discard changes (unstaged only)
      if ((e.key === "Delete" || e.key === "Backspace") && !isStaged) {
        e.preventDefault();
        const fileName = focused.path.split("/").pop() ?? focused.path;
        confirm(
          `Are you sure you want to discard changes in "${fileName}"?\n\nThis action is irreversible.`,
          { title: "Discard Changes", kind: "warning", okLabel: "Discard", cancelLabel: "Cancel" },
        ).then((confirmed) => {
          if (!confirmed) return;
          startOperation("discard");
          commands.discardChanges([focused.path])
            .then(setStatus)
            .catch((err) => setError(String(err)))
            .finally(() => endOperation("discard"));
        });
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("keydown", handleKeyDown);
      if (chordTimer) clearTimeout(chordTimer);
    };
  }, [
    setStatus, setError, setDiffMode, diffMode,
    setFocusedIndex, setSelectedFile, setDiff,
    clearFileSelection, startOperation, endOperation,
    mainView, setMainView,
  ]);
};
