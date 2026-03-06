import { useState, useCallback, useRef, useEffect } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { useRepositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";

interface CloneDialogProps {
  onClose: () => void;
}

export const CloneDialog = ({ onClose }: CloneDialogProps) => {
  const [url, setUrl] = useState("");
  const [directory, setDirectory] = useState("");
  const [cloning, setCloning] = useState(false);
  const { setStatus, setError } = useRepositoryStore();
  const inputRef = useRef<HTMLInputElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleBrowse = useCallback(async () => {
    const selected = await open({ directory: true, title: "Choose directory to clone into" });
    if (selected) {
      setDirectory(selected);
    }
  }, []);

  const handleClone = useCallback(async () => {
    if (!url.trim() || !directory.trim()) return;
    const repoName = url.trim().split("/").pop()?.replace(/\.git$/, "") ?? "repo";
    const targetPath = `${directory.trim()}/${repoName}`;
    setCloning(true);
    try {
      const status = await commands.cloneRepository(url.trim(), targetPath);
      setStatus(status);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setCloning(false);
    }
  }, [url, directory, setStatus, setError, onClose]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter") {
      handleClone();
    }
  }, [onClose, handleClone]);

  const handleOverlayClick = useCallback((e: React.MouseEvent) => {
    if (e.target === overlayRef.current) {
      onClose();
    }
  }, [onClose]);

  return (
    <div className="branch-picker-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="clone-dialog" onKeyDown={handleKeyDown}>
        <div className="clone-dialog-title">Clone Repository</div>
        <div className="clone-dialog-field">
          <label className="clone-dialog-label">Repository URL</label>
          <input
            ref={inputRef}
            className="clone-dialog-input"
            value={url}
            onChange={(e) => setUrl(e.target.value)}
            placeholder="https://github.com/user/repo.git"
          />
        </div>
        <div className="clone-dialog-field">
          <label className="clone-dialog-label">Directory</label>
          <div className="clone-dialog-dir-row">
            <input
              className="clone-dialog-input"
              value={directory}
              onChange={(e) => setDirectory(e.target.value)}
              placeholder="Select a directory..."
            />
            <button className="clone-dialog-browse" onClick={handleBrowse}>
              <span className="codicon codicon-folder-opened" />
            </button>
          </div>
        </div>
        <div className="clone-dialog-actions">
          <button className="action-button secondary" onClick={onClose} disabled={cloning}>
            Cancel
          </button>
          <button
            className="action-button"
            onClick={handleClone}
            disabled={!url.trim() || !directory.trim() || cloning}
          >
            {cloning ? "Cloning..." : "Clone"}
          </button>
        </div>
      </div>
    </div>
  );
};
