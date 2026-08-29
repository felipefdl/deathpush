import { createSignal } from "solid-js";
import { useRepository } from "../../hooks/use-repository";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";
import * as commands from "../../lib/tauri-commands";
import { ActionButton } from "./action-button";
import { OverflowMenu } from "./overflow-menu";

type ScmToolbarProps = {
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
};

export const ScmToolbar = (props: ScmToolbarProps) => {
  const { refreshStatus } = useRepository();
  const status = useStore(repositoryStore, (s) => s.status);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const { setStatus, setError, startOperation, endOperation } = repositoryStore.getState();
  const viewMode = useStore(layoutStore, (s) => s.viewMode);
  const { setViewMode } = layoutStore.getState();
  const [showOverflow, setShowOverflow] = createSignal(false);
  let overflowRef: HTMLButtonElement | undefined;

  const isStaging = () => operations().has("stage");

  const handleRefresh = () => {
    void refreshStatus();
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
    <div class="scm-toolbar">
      {status() && (
        <>
          <button
            class="scm-toolbar-button"
            onClick={() => setViewMode(viewMode() === "list" ? "tree" : "list")}
            title={viewMode() === "list" ? "View as Tree" : "View as List"}
          >
            <span class={`codicon ${viewMode() === "list" ? "codicon-list-tree" : "codicon-list-flat"}`} />
          </button>
          <button class="scm-toolbar-button" onClick={handleStageAll} disabled={isStaging()} title="Stage All Changes">
            <span class="codicon codicon-add" />
          </button>
          <button class="scm-toolbar-button" onClick={handleRefresh} title="Refresh">
            <span class="codicon codicon-refresh" />
          </button>
          <ActionButton />
          <div class="overflow-menu-wrapper">
            <button
              ref={(el) => {
                overflowRef = el;
              }}
              class="scm-toolbar-button"
              onClick={() => setShowOverflow(!showOverflow())}
              title="More Actions..."
            >
              <span class="codicon codicon-ellipsis" />
            </button>
            {showOverflow() && (
              <OverflowMenu
                anchorRef={overflowRef}
                onClose={() => setShowOverflow(false)}
                onOpenRepository={props.onOpenRepository}
                onCloneRepository={props.onCloneRepository}
              />
            )}
          </div>
        </>
      )}
    </div>
  );
};
