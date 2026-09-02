import { onSettled } from "solid-js";
import { repositoryStore } from "../stores/repository-store";
import { layoutStore } from "../stores/layout-store";
import { settingsStore } from "../stores/settings-store";
import { explorerStore } from "../stores/explorer-store";
import { toggleTerminal } from "../lib/toggle-terminal";
import { handleTerminalShortcut } from "../lib/terminal-shortcuts";
import { isPierreFindHostOpen } from "../lib/pierre/find-host";
import * as commands from "../lib/tauri-commands";
import { sendDestructiveIntent } from "../lib/session-client";
import { recoverFromSnapshot } from "./use-repository";
import { useDiff } from "./use-diff";

export const useKeyboardShortcuts = () => {
  const { clearDiff } = useDiff();
  onSettled(() => {
    let chordK = false;
    let chordTimer: ReturnType<typeof setTimeout> | null = null;

    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;
      const repo = repositoryStore.getState();
      const layout = layoutStore.getState();
      const { setError } = repo;

      // Chord: Cmd+K Cmd+T -> open theme picker
      if (chordK && isMod && e.key === "t") {
        e.preventDefault();
        chordK = false;
        if (chordTimer) clearTimeout(chordTimer);
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
        return;
      }

      if (isMod && e.key === "k") {
        e.preventDefault();
        chordK = true;
        if (chordTimer) clearTimeout(chordTimer);
        chordTimer = setTimeout(() => {
          chordK = false;
        }, 1500);
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
        settingsStore.getState().zoomIn();
        return;
      }
      if (isMod && e.key === "-") {
        e.preventDefault();
        settingsStore.getState().zoomOut();
        return;
      }
      if (isMod && e.key === "0") {
        e.preventDefault();
        settingsStore.getState().resetZoom();
        return;
      }

      // Opt+Cmd+1..9: Switch terminal tabs (check before Cmd+digit)
      if (isMod && e.altKey && e.code >= "Digit1" && e.code <= "Digit9") {
        e.preventDefault();
        const idx = parseInt(e.code.slice(5), 10) - 1;
        const group = repo.terminalGroups[idx];
        if (group) {
          repo.setActiveGroup(group.groupId);
        }
        return;
      }

      // Cmd+1: Changes, Cmd+2: Explorer, Cmd+3: Terminal
      if (isMod && !e.altKey && e.key === "1") {
        e.preventDefault();
        layout.setSidebarView("scm");
        layout.setMainView("changes");
        return;
      }
      if (isMod && !e.altKey && e.key === "2") {
        e.preventDefault();
        layout.setSidebarView("explorer");
        layout.setMainView("file");
        return;
      }
      if (isMod && !e.altKey && e.key === "3") {
        e.preventDefault();
        if (!layout.terminalVisible) {
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
        layout.setMainView(layout.mainView === "settings" ? "changes" : "settings");
        return;
      }

      if (isMod && e.key === "j") {
        e.preventDefault();
        toggleTerminal();
        return;
      }
      if (handleTerminalShortcut(e)) return;

      const target = e.target as HTMLElement;
      const isInput = target.tagName === "INPUT" || target.tagName === "TEXTAREA" || target.isContentEditable;

      if (isMod && e.shiftKey && e.key === "G") {
        e.preventDefault();
        void recoverFromSnapshot().catch((err) => setError(String(err)));
        return;
      }

      // Ctrl/Cmd+Shift+P: Toggle diff layout
      if (isMod && e.shiftKey && e.key === "P") {
        e.preventDefault();
        const { settings, updateDiff } = settingsStore.getState();
        updateDiff({ layout: settings.diff.layout === "inline" ? "sideBySide" : "inline" });
        return;
      }

      // Explorer shortcuts: only when explorer sidebar is active and focus is within explorer-view
      const inExplorer =
        layout.sidebarView === "explorer" && !isInput && !!document.activeElement?.closest(".explorer-view");

      if (inExplorer) {
        const explorer = explorerStore.getState();
        const selected = explorer.selectedTreeEntry;

        if (e.key === "F2" && selected) {
          e.preventDefault();
          window.dispatchEvent(new CustomEvent("deathpush:explorer-rename"));
          return;
        }

        if ((e.key === "Delete" || (isMod && e.key === "Backspace")) && selected) {
          e.preventDefault();
          void sendDestructiveIntent({ type: "deleteFile", path: selected.path, confirmed: false })
            .then((result) => {
              if (result.kind !== "snapshot") return;
              explorer.setSelectedTreeEntry(null);
              if (explorer.selectedPath === selected.path) {
                explorer.setSelectedPath(null);
                explorer.setFileContent(null);
              }
            })
            .catch((error) => setError(String(error)));
          return;
        }

        if (isMod && (e.key === "c" || e.key === "x") && selected) {
          e.preventDefault();
          explorer.setClipboardEntry({
            path: selected.path,
            isDirectory: selected.isDirectory,
            operation: e.key === "c" ? "copy" : "cut",
          });
          return;
        }

        if (isMod && e.key === "v" && explorer.clipboardEntry) {
          e.preventDefault();
          const clip = explorer.clipboardEntry;
          const separator = selected?.path.lastIndexOf("/") ?? -1;
          const targetDir = selected?.isDirectory
            ? selected.path
            : separator >= 0
              ? (selected?.path.slice(0, separator) ?? "")
              : "";
          const pasteOp =
            clip.operation === "copy"
              ? commands.copyEntries([clip.path], targetDir)
              : commands.moveEntries([clip.path], targetDir);
          pasteOp
            .then(() => {
              if (clip.operation === "cut") explorer.setClipboardEntry(null);
            })
            .catch((error) => setError(String(error)));
          return;
        }
      }

      if ((e.target as HTMLElement).closest("file-tree-container")) return;
      if (e.key === "Escape" && isPierreFindHostOpen()) return;

      // Skip navigation keys when focus is in an input
      if (isInput) return;

      // Escape: clear focus and selection
      if (e.key === "Escape") {
        e.preventDefault();
        clearDiff();
        const explorer = explorerStore.getState();
        explorer.setSelectedPath(null);
        explorer.setFileContent(null);
        return;
      }
    };

    window.addEventListener("keydown", handleKeyDown, true);
    return () => {
      window.removeEventListener("keydown", handleKeyDown, true);
      if (chordTimer) clearTimeout(chordTimer);
    };
  });
};
