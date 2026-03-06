import { useEffect, useState, useCallback, useRef } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import type { WorkspaceEntry } from "../../stores/settings-store";

interface WorkspaceConfigModalProps {
  onClose: () => void;
  workspaces: WorkspaceEntry[];
  onSave: (workspaces: WorkspaceEntry[]) => void;
}

const EMPTY_ENTRY: WorkspaceEntry = { directory: "", scanDepth: 1 };

export const WorkspaceConfigModal = ({ onClose, workspaces, onSave }: WorkspaceConfigModalProps) => {
  const [entries, setEntries] = useState<WorkspaceEntry[]>(
    workspaces.length > 0 ? workspaces.map((w) => ({ ...w })) : [{ ...EMPTY_ENTRY }],
  );
  const overlayRef = useRef<HTMLDivElement>(null);
  const listRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const firstInput = listRef.current?.querySelector<HTMLInputElement>(".clone-dialog-input");
    firstInput?.focus();
  }, []);

  const handleBrowse = useCallback(async (index: number) => {
    const selected = await open({ directory: true, title: "Select Git Projects Directory" });
    if (selected) {
      setEntries((prev) => prev.map((e, i) => (i === index ? { ...e, directory: selected } : e)));
    }
  }, []);

  const handleDirectoryChange = useCallback((index: number, value: string) => {
    setEntries((prev) => prev.map((e, i) => (i === index ? { ...e, directory: value } : e)));
  }, []);

  const handleDepthChange = useCallback((index: number, delta: number) => {
    setEntries((prev) =>
      prev.map((e, i) => (i === index ? { ...e, scanDepth: Math.min(5, Math.max(1, e.scanDepth + delta)) } : e)),
    );
  }, []);

  const handleRemove = useCallback((index: number) => {
    setEntries((prev) => prev.filter((_, i) => i !== index));
  }, []);

  const handleAdd = useCallback(() => {
    setEntries((prev) => [...prev, { ...EMPTY_ENTRY }]);
    requestAnimationFrame(() => {
      const inputs = listRef.current?.querySelectorAll<HTMLInputElement>(".clone-dialog-input");
      inputs?.[inputs.length - 1]?.focus();
    });
  }, []);

  const handleSave = useCallback(() => {
    const filtered = entries.filter((e) => e.directory.trim() !== "");
    onSave(filtered);
    onClose();
  }, [entries, onSave, onClose]);

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      } else if (e.key === "Enter") {
        handleSave();
      }
    },
    [onClose, handleSave],
  );

  const handleOverlayClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === overlayRef.current) {
        onClose();
      }
    },
    [onClose],
  );

  return (
    <div className="branch-picker-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="clone-dialog" onKeyDown={handleKeyDown}>
        <div className="clone-dialog-title">Workspace Settings</div>
        <div className="workspace-config-description">
          Add directories containing your Git repositories. The scan depth controls how many levels deep to search for
          projects within each directory.
        </div>
        <div className="workspace-entries" ref={listRef}>
          {entries.map((entry, index) => (
            <div key={index} className="workspace-entry-row">
              <input
                className="clone-dialog-input"
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
                data-form-type="other"
                value={entry.directory}
                onChange={(e) => handleDirectoryChange(index, e.target.value)}
                placeholder="Select a directory..."
              />
              <button className="clone-dialog-browse" onClick={() => handleBrowse(index)} title="Browse...">
                <span className="codicon codicon-folder-opened" />
              </button>
              <div className="welcome-depth-control">
                <button
                  className="welcome-depth-btn"
                  disabled={entry.scanDepth <= 1}
                  onClick={() => handleDepthChange(index, -1)}
                >
                  <span className="codicon codicon-chevron-left" />
                </button>
                <span className="welcome-depth-value">{entry.scanDepth}</span>
                <button
                  className="welcome-depth-btn"
                  disabled={entry.scanDepth >= 5}
                  onClick={() => handleDepthChange(index, 1)}
                >
                  <span className="codicon codicon-chevron-right" />
                </button>
              </div>
              {entries.length > 1 && (
                <button className="workspace-entry-remove" onClick={() => handleRemove(index)} title="Remove">
                  <span className="codicon codicon-close" />
                </button>
              )}
            </div>
          ))}
        </div>
        <button className="workspace-add-btn" onClick={handleAdd}>
          <span className="codicon codicon-add" />
          Add Directory
        </button>
        <div className="clone-dialog-actions">
          <button className="action-button secondary" onClick={onClose}>
            Cancel
          </button>
          <button className="action-button" onClick={handleSave}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
};
