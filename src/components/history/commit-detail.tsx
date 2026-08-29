import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import type * as monaco from "monaco-editor";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { settingsStore } from "../../stores/settings-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { formatRelativeDate } from "../../lib/format-date";
import { getCommitFileDiff } from "../../lib/tauri-commands";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import type { CommitDiffContent } from "../../lib/git-types";
import { applyDiffModelOptions } from "../../lib/monaco-models";
import { buildDiffModelOptions, buildDiffOptions } from "../../lib/diff-options";
import { ImageDiff } from "../diff/image-diff";
import { MonacoDiffEditor } from "../monaco/monaco-diff-editor";
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
  const diffMode = useStore(layoutStore, (s) => s.diffMode);
  const editorSettings = useStore(settingsStore, (s) => s.settings.editor);
  const currentTheme = useStore(themeStore, (s) => s.currentTheme);
  const [fileDiff, setFileDiff] = createSignal<CommitDiffContent | null>(null);
  const [selectedPath, setSelectedPath] = createSignal<string | null>(null);
  const [filesViewMode, setFilesViewMode] = createSignal<"list" | "tree">("list");
  let editorRef: monaco.editor.IStandaloneDiffEditor | undefined;
  let disposeActions: (() => void) | undefined;

  const commit = createMemo(() => commitDetail()?.commit);
  const files = createMemo(() => commitDetail()?.files ?? []);
  const firstLine = createMemo(() => commit()?.message.split("\n")[0] ?? "");
  const bodyLines = createMemo(() => commit()?.message.split("\n").slice(1).join("\n").trim() ?? "");
  const diffOptions = createMemo(() => ({
    ...buildDiffOptions(editorSettings(), diffMode()),
    readOnly: true,
    domReadOnly: true,
    tabSize: editorSettings().tabSize,
  }));

  const handleDiffMount = (editor: monaco.editor.IStandaloneDiffEditor, monacoApi: typeof monaco) => {
    editorRef = editor;
    disposeActions?.();

    const chordKT = monacoApi.KeyMod.chord(
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyK,
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyT
    );
    const chordKI = monacoApi.KeyMod.chord(
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyK,
      monacoApi.KeyMod.CtrlCmd | monacoApi.KeyCode.KeyI
    );

    const disposables: { dispose: () => void }[] = [];
    for (const sub of [editor.getModifiedEditor(), editor.getOriginalEditor()]) {
      disposables.push(
        sub.addAction({
          id: "deathpush.openThemePicker",
          label: "Open Theme Picker",
          keybindings: [chordKT],
          run: () => {
            window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
          },
        }),
        sub.addAction({
          id: "deathpush.openIconThemePicker",
          label: "Open Icon Theme Picker",
          keybindings: [chordKI],
          run: () => {
            window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
          },
        })
      );
    }

    disposeActions = () => {
      for (const disposable of disposables) disposable.dispose();
    };

    applyDiffModelOptions(editor, buildDiffModelOptions(settingsStore.getState().settings.editor));
  };

  createEffect(
    () => editorSettings(),
    (editor) => {
      const instance = editorRef;
      if (!instance) return;
      applyDiffModelOptions(instance, buildDiffModelOptions(editor));
    }
  );

  onSettled(() => {
    return () => {
      disposeActions?.();
      editorRef = undefined;
    };
  });

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
                  <MonacoDiffEditor
                    original={fileDiff()!.original}
                    modified={fileDiff()!.modified}
                    originalPath={`commit-original/${fileDiff()!.path}`}
                    modifiedPath={`commit-modified/${fileDiff()!.path}`}
                    language={fileDiff()!.language ?? undefined}
                    theme={currentTheme().id}
                    onMount={handleDiffMount}
                    options={diffOptions()}
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
