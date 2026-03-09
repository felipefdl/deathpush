import { useCallback } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useExplorerStore } from "../../stores/explorer-store";
import { useTauriEvent } from "../../hooks/use-tauri-event";
import { ExplorerTree } from "./explorer-tree";
import * as commands from "../../lib/tauri-commands";
import "../../styles/explorer.css";

interface ExplorerViewProps {
  onOpenRepository: () => void;
}

export const ExplorerView = ({ onOpenRepository }: ExplorerViewProps) => {
  const status = useRepositoryStore((s) => s.status);
  const { clearCache, fileFilter, setFileFilter } = useExplorerStore();

  const handleRefresh = useCallback(() => {
    clearCache();
  }, [clearCache]);

  // Listen for repository-changed events to refresh expanded dirs
  useTauriEvent("repository-changed", useCallback(() => {
    const store = useExplorerStore.getState();
    // Re-fetch the root
    const rootKey = "__root__";
    commands.listDirectory(null).then((result) => {
      store.setDirectoryEntries(rootKey, result);
    }).catch(() => {});
    // Re-fetch expanded dirs
    for (const dir of store.expandedDirs) {
      commands.listDirectory(dir).then((result) => {
        store.setDirectoryEntries(dir, result);
      }).catch(() => {});
    }
  }, []));

  if (!status) {
    return (
      <div className="explorer-view">
        <div className="explorer-header">
          <span className="explorer-header-title">Explorer</span>
        </div>
        <div className="scm-empty">
          <span style={{ opacity: 0.5, padding: 16, display: "block", textAlign: "center" }}>
            No repository open
          </span>
          <button
            className="action-button"
            style={{ margin: "0 16px", width: "auto" }}
            onClick={onOpenRepository}
          >
            <span className="codicon codicon-folder-opened" />
            Open Repository
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="explorer-view">
      <div className="explorer-header">
        <div className="explorer-filter">
          <span className="codicon codicon-search explorer-filter-icon" />
          <input
            className="explorer-filter-input"
            type="text"
            placeholder="Filter files..."
            value={fileFilter}
            onChange={(e) => setFileFilter(e.target.value)}
            autoComplete="off"
            autoCorrect="off"
            autoCapitalize="off"
            spellCheck={false}
          />
          {fileFilter && (
            <button className="explorer-filter-clear" onClick={() => setFileFilter("")}>
              <span className="codicon codicon-close" />
            </button>
          )}
        </div>
        <div className="explorer-header-actions">
          <button className="scm-toolbar-button" onClick={handleRefresh} title="Refresh Explorer">
            <span className="codicon codicon-refresh" />
          </button>
        </div>
      </div>
      <div className="explorer-tree">
        <ExplorerTree />
      </div>
    </div>
  );
};
