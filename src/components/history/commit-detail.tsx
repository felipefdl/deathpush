import { createMemo, createSignal, For } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { formatRelativeDate } from "../../lib/format-date";
import { getCommitFileDiff } from "../../lib/tauri-commands";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import type { CommitDiffContent } from "../../lib/git-types";
import { ImageDiff } from "../diff/image-diff";
import { PierreFileDiff, historyCacheKey } from "../pierre/pierre-file-diff";
import { CommitFileTree } from "./commit-file-tree";

const statusLetter = (status: string): string => {
  switch (status) {
    case "added":
      return "A";
    case "deleted":
      return "D";
    case "modified":
      return "M";
    case "renamed":
      return "R";
    case "copied":
      return "C";
    case "typeChanged":
      return "T";
    default:
      return "M";
  }
};

const copyToClipboard = (text: string) => {
  void navigator.clipboard.writeText(text);
};

export const CommitDetail = () => {
  const commitDetail = useStore(repositoryStore, (s) => s.commitDetail);
  const [fileDiff, setFileDiff] = createSignal<CommitDiffContent | null>(null);
  const [selectedPath, setSelectedPath] = createSignal<string | null>(null);
  const [filesViewMode, setFilesViewMode] = createSignal<"list" | "tree">("list");

  const commit = createMemo(() => commitDetail()?.commit);
  const files = createMemo(() => commitDetail()?.files ?? []);
  const firstLine = createMemo(() => commit()?.message.split("\n")[0] ?? "");
  const bodyLines = createMemo(() => commit()?.message.split("\n").slice(1).join("\n").trim() ?? "");
  const handleFileClick = async (commitId: string, path: string) => {
    setSelectedPath(path);
    try {
      const diff = await getCommitFileDiff(commitId, path);
      setFileDiff(diff);
    } catch {
      setFileDiff(null);
    }
  };

  return (
    <>
      {!commitDetail() ? (
        <div class="commit-detail-empty">
          <span class="codicon codicon-history commit-detail-empty-icon" />
          <span>Select a commit to view details</span>
        </div>
      ) : (
        <div class="commit-detail">
          <div class="commit-detail-header">
            <div class="commit-detail-meta-inline">
              <span class="commit-detail-title">{firstLine()}</span>
              <span class="commit-meta-id">{commit()!.shortId}</span>
              <span class="commit-meta-separator">&middot;</span>
              <span>{commit()!.authorName}</span>
              <span class="commit-meta-separator">&middot;</span>
              <span>{formatRelativeDate(commit()!.authorDate)}</span>
              <span class="commit-meta-actions">
                <button class="commit-copy-btn" onClick={() => copyToClipboard(commit()!.id)} title="Copy full SHA">
                  <span class="codicon codicon-copy" />
                </button>
                <button
                  class="commit-copy-btn"
                  onClick={() => copyToClipboard(commit()!.message)}
                  title="Copy commit message"
                >
                  <span class="codicon codicon-comment" />
                </button>
                <button
                  class="commit-copy-btn"
                  onClick={() => copyToClipboard(commit()!.authorEmail)}
                  title="Copy email"
                >
                  <span class="codicon codicon-mail" />
                </button>
              </span>
            </div>
            {bodyLines() && <div class="commit-detail-body">{bodyLines()}</div>}
            {commit()!.parentIds.length > 1 && (
              <div class="commit-detail-parents">
                Merge:{" "}
                {commit()!
                  .parentIds.map((parent) => parent.slice(0, 7))
                  .join(", ")}
              </div>
            )}
          </div>
          <div class="commit-detail-files">
            <div class="commit-detail-files-header">
              <span class="commit-detail-files-header-label">Changed Files ({files().length})</span>
              <button
                class="scm-toolbar-button"
                onClick={() => setFilesViewMode((mode) => (mode === "list" ? "tree" : "list"))}
                title={filesViewMode() === "list" ? "Show as tree" : "Show as list"}
              >
                <span class={["codicon", filesViewMode() === "list" ? "codicon-list-tree" : "codicon-list-flat"]} />
              </button>
            </div>
            {filesViewMode() === "tree" ? (
              <CommitFileTree
                files={files()}
                selectedPath={selectedPath()}
                onFileClick={(path) => handleFileClick(commit()!.id, path)}
              />
            ) : (
              <For each={files()} keyed={(file) => file.path}>
                {(file) => (
                  <div
                    class={["commit-detail-file", { selected: selectedPath() === file().path }]}
                    onClick={() => handleFileClick(commit()!.id, file().path)}
                  >
                    <span class={["commit-detail-file-icon", getFileIconClasses(file().path, "file")]} />
                    <span class="commit-detail-file-path" title={file().path}>
                      {file().oldPath ? `${file().oldPath} -> ${file().path}` : file().path}
                    </span>
                    <span class={["commit-file-badge", `badge-${file().status}`]}>{statusLetter(file().status)}</span>
                  </div>
                )}
              </For>
            )}
          </div>
          {fileDiff() && (
            <div class="commit-detail-diff">
              <div class="commit-detail-diff-header">{fileDiff()!.path}</div>
              {fileDiff()!.fileType === "image" ? (
                <ImageDiff original={fileDiff()!.original} modified={fileDiff()!.modified} />
              ) : (
                <div class="commit-detail-diff-editor">
                  <PierreFileDiff
                    path={fileDiff()!.path}
                    original={fileDiff()!.original}
                    modified={fileDiff()!.modified}
                    cacheKey={historyCacheKey(commit()?.id ?? "history", fileDiff()!.path)}
                  />
                </div>
              )}
            </div>
          )}
        </div>
      )}
    </>
  );
};
