import { useCallback, useState } from "react";
import { DiffEditor, type DiffOnMount } from "@monaco-editor/react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { formatRelativeDate } from "../../lib/format-date";
import { getCommitFileDiff } from "../../lib/tauri-commands";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import type { CommitDiffContent } from "../../lib/git-types";
import { buildDiffOptions } from "../../lib/diff-options";
import { ImageDiff } from "../diff/image-diff";
import { CommitFileTree } from "./commit-file-tree";

const statusLetter = (status: string): string => {
  switch (status) {
    case "added": return "A";
    case "deleted": return "D";
    case "modified": return "M";
    case "renamed": return "R";
    case "copied": return "C";
    case "typeChanged": return "T";
    default: return "M";
  }
};

const copyToClipboard = (text: string) => {
  navigator.clipboard.writeText(text);
};

export const CommitDetail = () => {
  const { commitDetail } = useRepositoryStore();
  const { diffMode } = useLayoutStore();
  const { settings } = useSettingsStore();
  const { currentTheme } = useThemeStore();
  const [fileDiff, setFileDiff] = useState<CommitDiffContent | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);
  const [filesViewMode, setFilesViewMode] = useState<"list" | "tree">("list");

  const handleDiffMount: DiffOnMount = useCallback((_editor, monaco) => {
    const chordKT = monaco.KeyMod.chord(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyT,
    );
    const chordKI = monaco.KeyMod.chord(
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyK,
      monaco.KeyMod.CtrlCmd | monaco.KeyCode.KeyI,
    );

    for (const sub of [_editor.getModifiedEditor(), _editor.getOriginalEditor()]) {
      sub.addAction({
        id: "deathpush.openThemePicker",
        label: "Open Theme Picker",
        keybindings: [chordKT],
        run: () => {
          window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
        },
      });
      sub.addAction({
        id: "deathpush.openIconThemePicker",
        label: "Open Icon Theme Picker",
        keybindings: [chordKI],
        run: () => {
          window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
        },
      });
    }
  }, []);

  const handleFileClick = useCallback(async (commitId: string, path: string) => {
    setSelectedPath(path);
    try {
      const diff = await getCommitFileDiff(commitId, path);
      setFileDiff(diff);
    } catch {
      setFileDiff(null);
    }
  }, []);

  if (!commitDetail) {
    return (
      <div className="commit-detail-empty">
        <span className="codicon codicon-history commit-detail-empty-icon" />
        <span>Select a commit to view details</span>
      </div>
    );
  }

  const { commit, files } = commitDetail;
  const firstLine = commit.message.split("\n")[0];
  const bodyLines = commit.message.split("\n").slice(1).join("\n").trim();

  return (
    <div className="commit-detail">
      <div className="commit-detail-header">
        <div className="commit-detail-meta-inline">
          <span className="commit-detail-title">{firstLine}</span>
          <span className="commit-meta-id">{commit.shortId}</span>
          <span className="commit-meta-separator">&middot;</span>
          <span>{commit.authorName}</span>
          <span className="commit-meta-separator">&middot;</span>
          <span>{formatRelativeDate(commit.authorDate)}</span>
          <span className="commit-meta-actions">
            <button
              className="commit-copy-btn"
              onClick={() => copyToClipboard(commit.id)}
              title="Copy full SHA"
            >
              <span className="codicon codicon-copy" />
            </button>
            <button
              className="commit-copy-btn"
              onClick={() => copyToClipboard(commit.message)}
              title="Copy commit message"
            >
              <span className="codicon codicon-comment" />
            </button>
            <button
              className="commit-copy-btn"
              onClick={() => copyToClipboard(commit.authorEmail)}
              title="Copy email"
            >
              <span className="codicon codicon-mail" />
            </button>
          </span>
        </div>
        {bodyLines && <div className="commit-detail-body">{bodyLines}</div>}
        {commit.parentIds.length > 1 && (
          <div className="commit-detail-parents">
            Merge: {commit.parentIds.map((p) => p.slice(0, 7)).join(", ")}
          </div>
        )}
      </div>
      <div className="commit-detail-files">
        <div className="commit-detail-files-header">
          <span className="commit-detail-files-header-label">
            Changed Files ({files.length})
          </span>
          <button
            className="scm-toolbar-button"
            onClick={() => setFilesViewMode(filesViewMode === "list" ? "tree" : "list")}
            title={filesViewMode === "list" ? "Show as tree" : "Show as list"}
          >
            <span className={`codicon codicon-${filesViewMode === "list" ? "list-tree" : "list-flat"}`} />
          </button>
        </div>
        {filesViewMode === "tree" ? (
          <CommitFileTree
            files={files}
            selectedPath={selectedPath}
            onFileClick={(path) => handleFileClick(commit.id, path)}
          />
        ) : (
          files.map((file) => (
            <div
              key={file.path}
              className={`commit-detail-file${selectedPath === file.path ? " selected" : ""}`}
              onClick={() => handleFileClick(commit.id, file.path)}
            >
              <span className={`commit-detail-file-icon ${getFileIconClasses(file.path, "file")}`} />
              <span className="commit-detail-file-path" title={file.path}>
                {file.oldPath ? `${file.oldPath} -> ${file.path}` : file.path}
              </span>
              <span className={`commit-file-badge badge-${file.status}`}>
                {statusLetter(file.status)}
              </span>
            </div>
          ))
        )}
      </div>
      {fileDiff && (
        <div className="commit-detail-diff">
          <div className="commit-detail-diff-header">{fileDiff.path}</div>
          {fileDiff.fileType === "image" ? (
            <ImageDiff original={fileDiff.original} modified={fileDiff.modified} />
          ) : (
            <div className="commit-detail-diff-editor">
              <DiffEditor
                original={fileDiff.original}
                modified={fileDiff.modified}
                language={fileDiff.language ?? undefined}
                theme={currentTheme.id}
                keepCurrentOriginalModel
                keepCurrentModifiedModel
                onMount={handleDiffMount}
                options={{
                  ...buildDiffOptions(settings.editor, diffMode),
                  readOnly: true,
                  domReadOnly: true,
                }}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
};
