import { useCallback, useEffect, useRef } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useLayoutStore } from "../../stores/layout-store";
import * as commands from "../../lib/tauri-commands";

export const DiffHeader = ({ isDirty = false }: { isDirty?: boolean }) => {
  const { selectedFile, isDiffDirty, setBlame } = useRepositoryStore();
  const blameEnabled = useSettingsStore((s) => s.settings.git.blame);
  const { diffMode, setDiffMode, setMainView } = useLayoutStore();
  const fetchedPathRef = useRef<string | null>(null);

  // Fetch blame when enabled in settings
  useEffect(() => {
    if (!blameEnabled || !selectedFile || selectedFile.staged || isDiffDirty) {
      fetchedPathRef.current = null;
      setBlame(null);
      return;
    }
    if (fetchedPathRef.current === selectedFile.path) return;
    fetchedPathRef.current = selectedFile.path;
    commands.getFileBlame(selectedFile.path).then(setBlame).catch(() => setBlame(null));
  }, [blameEnabled, selectedFile?.path, selectedFile?.staged, isDiffDirty, setBlame]);

  const handleShowFileHistory = useCallback(() => {
    if (!selectedFile) return;
    setMainView("history");
    window.dispatchEvent(
      new CustomEvent("deathpush:file-history", { detail: { path: selectedFile.path } }),
    );
  }, [selectedFile, setMainView]);

  if (!selectedFile) return null;

  const fileName = selectedFile.path.split("/").pop() ?? selectedFile.path;
  const label = selectedFile.staged ? "Staged" : "Working Tree";

  return (
    <div className="diff-header">
      <span className="diff-header-path" title={selectedFile.path}>
        {fileName}
        {isDirty && <span className="diff-header-dirty"> *</span>}
        <span className="diff-header-label"> ({label})</span>
      </span>
      <div className="diff-header-actions">
        <button
          className="scm-toolbar-button"
          onClick={handleShowFileHistory}
          title="Show File History"
        >
          <span className="codicon codicon-history" />
        </button>
        <button
          className="scm-toolbar-button"
          onClick={() => setDiffMode(diffMode === "inline" ? "sideBySide" : "inline")}
          title={diffMode === "inline" ? "Switch to side by side" : "Switch to inline"}
        >
          <span className={`codicon ${diffMode === "inline" ? "codicon-split-horizontal" : "codicon-list-flat"}`} />
        </button>
      </div>
    </div>
  );
};
