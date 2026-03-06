import { useCallback } from "react";
import { DiffEditor, type DiffOnMount } from "@monaco-editor/react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { formatRelativeDate } from "../../lib/format-date";
import { getCommitFileDiff } from "../../lib/tauri-commands";
import { useState } from "react";
import type { CommitDiffContent } from "../../lib/git-types";
import { ImageDiff } from "../diff/image-diff";

export const CommitDetail = () => {
  const { commitDetail } = useRepositoryStore();
  const { diffMode } = useLayoutStore();
  const { settings } = useSettingsStore();
  const { currentTheme } = useThemeStore();
  const [fileDiff, setFileDiff] = useState<CommitDiffContent | null>(null);
  const [selectedPath, setSelectedPath] = useState<string | null>(null);

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
    return <div className="commit-detail-empty">Select a commit to view details.</div>;
  }

  const { commit, files } = commitDetail;
  const firstLine = commit.message.split("\n")[0];
  const bodyLines = commit.message.split("\n").slice(1).join("\n").trim();

  return (
    <div className="commit-detail">
      <div className="commit-detail-header">
        <div className="commit-detail-title">{firstLine}</div>
        {bodyLines && <div className="commit-detail-body">{bodyLines}</div>}
        <div className="commit-detail-meta">
          <span className="commit-detail-id">{commit.shortId}</span>
          <span className="commit-detail-author">{commit.authorName} &lt;{commit.authorEmail}&gt;</span>
          <span className="commit-detail-date">{formatRelativeDate(commit.authorDate)}</span>
        </div>
        {commit.parentIds.length > 1 && (
          <div className="commit-detail-parents">
            Merge: {commit.parentIds.map((p) => p.slice(0, 7)).join(", ")}
          </div>
        )}
      </div>
      <div className="commit-detail-files">
        <div className="commit-detail-files-header">
          Changed Files ({files.length})
        </div>
        {files.map((file) => (
          <div
            key={file.path}
            className={`commit-detail-file${selectedPath === file.path ? " selected" : ""}`}
            onClick={() => handleFileClick(commit.id, file.path)}
          >
            <span className={`commit-detail-file-status status-${file.status}`}>
              {statusLetter(file.status)}
            </span>
            <span className="commit-detail-file-path" title={file.path}>
              {file.oldPath ? `${file.oldPath} -> ${file.path}` : file.path}
            </span>
          </div>
        ))}
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
                onMount={handleDiffMount}
                options={{
                  readOnly: true,
                  renderSideBySide: diffMode === "sideBySide",
                  minimap: { enabled: settings.editor.minimap },
                  scrollBeyondLastLine: false,
                  fontSize: settings.editor.fontSize,
                  fontFamily: settings.editor.fontFamily,
                  lineHeight: settings.editor.lineHeight,
                  // @ts-expect-error tabSize works at runtime but is missing from IDiffEditorConstructionOptions
                  tabSize: settings.editor.tabSize,
                  wordWrap: settings.editor.wordWrap,
                  renderWhitespace: settings.editor.renderWhitespace,
                  renderOverviewRuler: false,
                  originalEditable: false,
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
