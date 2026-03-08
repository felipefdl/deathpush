import { useEffect, useRef } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useStash } from "../../hooks/use-stash";
import * as commands from "../../lib/tauri-commands";

interface OverflowMenuProps {
  anchorRef: React.RefObject<HTMLButtonElement | null>;
  onClose: () => void;
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
}

export const OverflowMenu = ({ anchorRef, onClose, onOpenRepository, onCloneRepository }: OverflowMenuProps) => {
  const menuRef = useRef<HTMLDivElement>(null);
  const { status, stashes, setStatus, setError, startOperation, endOperation, operations } = useRepositoryStore();
  const { viewMode, setViewMode } = useLayoutStore();
  const { saveStash, popStash } = useStash();

  const branch = status?.headBranch;
  const hasStaged = status?.groups.some((g) => g.kind === "index" && g.files.length > 0) ?? false;
  const hasUnstaged = status?.groups.some((g) => g.kind !== "index" && g.files.length > 0) ?? false;
  const hasCommit = !!status?.headCommit;
  const hasStashes = stashes.length > 0;

  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        menuRef.current && !menuRef.current.contains(e.target as Node) &&
        anchorRef.current && !anchorRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  }, [onClose, anchorRef]);

  const handleItem = (action: () => void, disabled?: boolean) => {
    if (disabled) return;
    onClose();
    action();
  };

  const handlePull = async () => {
    if (!branch) return;
    startOperation("pull");
    try {
      const newStatus = await commands.pull("origin", branch);
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("pull");
    }
  };

  const handlePush = async () => {
    if (!branch) return;
    startOperation("push");
    try {
      const newStatus = await commands.push("origin", branch);
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("push");
    }
  };

  const handleFetch = async () => {
    startOperation("fetch");
    try {
      const newStatus = await commands.fetchRemote("origin", true);
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("fetch");
    }
  };

  const handleStageAll = async () => {
    startOperation("stage");
    try {
      const newStatus = await commands.stageAll();
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstageAll = async () => {
    startOperation("unstage");
    try {
      const newStatus = await commands.unstageAll();
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscardAll = async () => {
    if (!status) return;
    const unstaged = status.groups.filter((g) => g.kind !== "index");
    const paths = unstaged.flatMap((g) => g.files.map((f) => f.path));
    const count = paths.length;
    if (count === 0) return;
    const confirmed = await confirm(
      `Are you sure you want to discard all ${count} change(s)?\n\nThis action is irreversible.`,
      { title: "Discard All Changes", kind: "warning", okLabel: "Discard All", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    startOperation("discard");
    try {
      const newStatus = await commands.discardChanges(paths);
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  };

  const handleUndoLastCommit = async () => {
    const confirmed = await confirm("Undo last commit? Changes will be moved back to staging.", {
      title: "Undo Last Commit",
      kind: "warning",
    });
    if (!confirmed) return;
    try {
      const newStatus = await commands.undoLastCommit();
      setStatus(newStatus);
    } catch (err) {
      setError(String(err));
    }
  };

  const noBranch = !branch;
  const isNetworkBusy = operations.has("push") || operations.has("pull") || operations.has("fetch");

  return (
    <div className="overflow-menu" ref={menuRef}>
      <div
        className="context-menu-item"
        onClick={() => handleItem(() => setViewMode(viewMode === "list" ? "tree" : "list"))}
      >
        <span
          className={`codicon ${viewMode === "list" ? "codicon-list-tree" : "codicon-list-flat"}`}
          style={{ marginRight: 8, fontSize: 14 }}
        />
        {viewMode === "list" ? "View as Tree" : "View as List"}
      </div>

      <div className="context-menu-separator" />

      <div
        className={`context-menu-item${noBranch || isNetworkBusy ? " disabled" : ""}`}
        onClick={() => handleItem(handlePull, noBranch || isNetworkBusy)}
      >
        Pull
      </div>
      <div
        className={`context-menu-item${noBranch || isNetworkBusy ? " disabled" : ""}`}
        onClick={() => handleItem(handlePush, noBranch || isNetworkBusy)}
      >
        Push
      </div>
      <div
        className={`context-menu-item${isNetworkBusy ? " disabled" : ""}`}
        onClick={() => handleItem(handleFetch, isNetworkBusy)}
      >
        Fetch
      </div>

      <div className="context-menu-separator" />

      <div
        className="context-menu-item"
        onClick={() => handleItem(handleStageAll)}
      >
        Stage All Changes
      </div>
      <div
        className={`context-menu-item${!hasStaged ? " disabled" : ""}`}
        onClick={() => handleItem(handleUnstageAll, !hasStaged)}
      >
        Unstage All Changes
      </div>
      <div
        className={`context-menu-item${!hasUnstaged ? " disabled" : ""}`}
        onClick={() => handleItem(handleDiscardAll, !hasUnstaged)}
      >
        Discard All Changes
      </div>

      <div className="context-menu-separator" />

      <div
        className="context-menu-item"
        onClick={() => handleItem(() => saveStash())}
      >
        Stash Changes
      </div>
      <div
        className={`context-menu-item${!hasStashes ? " disabled" : ""}`}
        onClick={() => handleItem(() => popStash(0), !hasStashes)}
      >
        Stash Pop (Latest)
      </div>

      <div className="context-menu-separator" />

      <div
        className={`context-menu-item${!hasCommit ? " disabled" : ""}`}
        onClick={() => handleItem(handleUndoLastCommit, !hasCommit)}
      >
        Undo Last Commit
      </div>

      <div className="context-menu-separator" />

      <div className="context-menu-item" onClick={() => handleItem(onOpenRepository)}>
        Open Repository...
      </div>
      {onCloneRepository && (
        <div className="context-menu-item" onClick={() => handleItem(onCloneRepository)}>
          Clone Repository...
        </div>
      )}
    </div>
  );
};
