import { useEffect } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../stores/repository-store";
import { useLayoutStore } from "../stores/layout-store";
import { useSettingsStore } from "../stores/settings-store";
import { useExplorerStore } from "../stores/explorer-store";
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

      // Quick Open: Cmd+P
      if (isMod && e.key === "p" && !e.shiftKey) {
        e.preventDefault();
        window.dispatchEvent(new CustomEvent("deathpush:open-quick-open"));
        return;
      }

      // Zoom: Cmd/Ctrl + =/- /0 (must be before isInput check)
      if (isMod && (e.key === "=" || e.key === "+")) {
        e.preventDefault();
        useSettingsStore.getState().zoomIn();
        return;
      }
      if (isMod && e.key === "-") {
        e.preventDefault();
        useSettingsStore.getState().zoomOut();
        return;
      }
      if (isMod && e.key === "0") {
        e.preventDefault();
        useSettingsStore.getState().resetZoom();
        return;
      }

      // Opt+Cmd+1..9: Switch terminal tabs (check before Cmd+digit)
      if (isMod && e.altKey && e.code >= "Digit1" && e.code <= "Digit9") {
        e.preventDefault();
        const idx = parseInt(e.code.slice(5), 10) - 1;
        const repo = useRepositoryStore.getState();
        const group = repo.terminalGroups[idx];
        if (group) {
          repo.setActiveGroup(group.groupId);
        }
        return;
      }

      // Cmd+1: Changes, Cmd+2: Explorer, Cmd+3: Terminal
      if (isMod && !e.altKey && e.key === "1") {
        e.preventDefault();
        const layout = useLayoutStore.getState();
        layout.setSidebarView("scm");
        layout.setMainView("changes");
        return;
      }
      if (isMod && !e.altKey && e.key === "2") {
        e.preventDefault();
        const layout = useLayoutStore.getState();
        layout.setSidebarView("explorer");
        layout.setMainView("file");
        return;
      }
      if (isMod && !e.altKey && e.key === "3") {
        e.preventDefault();
        const layout = useLayoutStore.getState();
        const repo = useRepositoryStore.getState();
        if (layout.terminalMaximized) {
          layout.setMainView("terminal");
          requestAnimationFrame(() => {
            window.dispatchEvent(new CustomEvent("deathpush:focus-terminal"));
          });
        } else if (!layout.terminalVisible) {
          if (repo.terminalGroups.length === 0) {
            repo.addTerminalGroup();
          }
          layout.setTerminalVisible(true);
          requestAnimationFrame(() => {
            window.dispatchEvent(new CustomEvent("deathpush:focus-terminal"));
          });
        } else {
          window.dispatchEvent(new CustomEvent("deathpush:focus-terminal"));
        }
        return;
      }

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

      if (isMod && e.key === "j") {
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

      // Explorer shortcuts: only when explorer sidebar is active and focus is within explorer-view
      const inExplorer =
        useLayoutStore.getState().sidebarView === "explorer" &&
        !isInput &&
        !!document.activeElement?.closest(".explorer-view");

      if (inExplorer) {
        const explorer = useExplorerStore.getState();
        const selectedPath = explorer.selectedPath;

        // F2 or Enter: start rename
        if ((e.key === "F2" || e.key === "Enter") && selectedPath) {
          e.preventDefault();
          explorer.setRenamingPath(selectedPath);
          return;
        }

        // Delete / Cmd+Backspace: move to trash
        if ((e.key === "Delete" || (isMod && e.key === "Backspace")) && selectedPath) {
          e.preventDefault();
          const fileName = selectedPath.split("/").pop() ?? selectedPath;
          confirm(
            `Are you sure you want to delete "${fileName}"?\n\nThis will move it to the trash.`,
            { title: "Delete", kind: "warning", okLabel: "Move to Trash", cancelLabel: "Cancel" },
          ).then((confirmed) => {
            if (!confirmed) return;
            commands.deleteFile(selectedPath).then((status) => {
              setStatus(status);
              explorer.setSelectedPath(null);
              explorer.setFileContent(null);
              explorer.clearCache();
            }).catch((err) => setError(String(err)));
          });
          return;
        }

        // Cmd+C: copy
        if (isMod && e.key === "c" && selectedPath) {
          e.preventDefault();
          // Determine if directory from cache
          const rootEntries = explorer.directoryCache.get("__root__");
          const findEntry = (entries: typeof rootEntries, path: string): boolean => {
            if (!entries) return false;
            for (const entry of entries) {
              if (entry.path === path) return entry.isDirectory;
              if (entry.isDirectory && path.startsWith(entry.path + "/")) {
                const children = explorer.directoryCache.get(entry.path);
                if (children) return findEntry(children, path);
              }
            }
            return false;
          };
          const isDir = findEntry(rootEntries, selectedPath);
          explorer.setClipboardEntry({ path: selectedPath, isDirectory: isDir, operation: "copy" });
          return;
        }

        // Cmd+X: cut
        if (isMod && e.key === "x" && selectedPath) {
          e.preventDefault();
          const rootEntries = explorer.directoryCache.get("__root__");
          const findEntry = (entries: typeof rootEntries, path: string): boolean => {
            if (!entries) return false;
            for (const entry of entries) {
              if (entry.path === path) return entry.isDirectory;
              if (entry.isDirectory && path.startsWith(entry.path + "/")) {
                const children = explorer.directoryCache.get(entry.path);
                if (children) return findEntry(children, path);
              }
            }
            return false;
          };
          const isDir = findEntry(rootEntries, selectedPath);
          explorer.setClipboardEntry({ path: selectedPath, isDirectory: isDir, operation: "cut" });
          return;
        }

        // Cmd+V: paste
        if (isMod && e.key === "v" && explorer.clipboardEntry) {
          e.preventDefault();
          const clip = explorer.clipboardEntry;
          // Paste into selected folder, or root
          const targetDir = selectedPath ?? "";
          const pasteOp = clip.operation === "copy"
            ? commands.copyEntries([clip.path], targetDir)
            : commands.moveEntries([clip.path], targetDir);
          pasteOp.then(() => {
            if (clip.operation === "cut") explorer.setClipboardEntry(null);
            explorer.clearCache();
          }).catch((err) => setError(String(err)));
          return;
        }
      }

      // Skip navigation keys when focus is in an input
      if (isInput) return;

      // Escape: clear focus and selection
      if (e.key === "Escape") {
        e.preventDefault();
        setFocusedIndex(null);
        setSelectedFile(null);
        setDiff(null);
        clearFileSelection();
        const explorer = useExplorerStore.getState();
        explorer.setSelectedPath(null);
        explorer.setFileContent(null);
        return;
      }

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
