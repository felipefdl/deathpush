import { useRef, useState } from "react";
import { useRepository } from "../../hooks/use-repository";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import * as commands from "../../lib/tauri-commands";
import { ActionButton } from "./action-button";
import { OverflowMenu } from "./overflow-menu";

interface ScmToolbarProps {
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
}

export const ScmToolbar = ({ onOpenRepository, onCloneRepository }: ScmToolbarProps) => {
  const { refreshStatus } = useRepository();
  const { setStatus, setError, status, startOperation, endOperation, operations } = useRepositoryStore();
  const { viewMode, setViewMode } = useLayoutStore();
  const [showOverflow, setShowOverflow] = useState(false);
  const overflowRef = useRef<HTMLButtonElement>(null);

  const isStaging = operations.has("stage");

  const handleRefresh = () => {
    refreshStatus();
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

  return (
    <div className="scm-toolbar">
      {status && (
        <>
          <button
            className="scm-toolbar-button"
            onClick={() => setViewMode(viewMode === "list" ? "tree" : "list")}
            title={viewMode === "list" ? "View as Tree" : "View as List"}
          >
            <span className={`codicon ${viewMode === "list" ? "codicon-list-tree" : "codicon-list-flat"}`} />
          </button>
          <button className="scm-toolbar-button" onClick={handleStageAll} disabled={isStaging} title="Stage All Changes">
            <span className="codicon codicon-add" />
          </button>
          <button className="scm-toolbar-button" onClick={handleRefresh} title="Refresh">
            <span className="codicon codicon-refresh" />
          </button>
          <ActionButton />
          <div className="overflow-menu-wrapper">
            <button
              ref={overflowRef}
              className="scm-toolbar-button"
              onClick={() => setShowOverflow(!showOverflow)}
              title="More Actions..."
            >
              <span className="codicon codicon-ellipsis" />
            </button>
            {showOverflow && (
              <OverflowMenu
                anchorRef={overflowRef}
                onClose={() => setShowOverflow(false)}
                onOpenRepository={onOpenRepository}
                onCloneRepository={onCloneRepository}
              />
            )}
          </div>
        </>
      )}
    </div>
  );
};
