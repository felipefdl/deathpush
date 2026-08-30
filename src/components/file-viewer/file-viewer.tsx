import { createEffect, createMemo, createSignal, For } from "solid-js";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { useColorScheme } from "../../hooks/use-color-scheme";
import { useDiskGuard } from "../../hooks/use-disk-guard";
import * as commands from "../../lib/tauri-commands";
import { sessionCacheKey, type SaveSession } from "../../lib/pierre/save-session";
import { sha256Utf8 } from "../../lib/pierre/sha";
import { useStore } from "../../lib/use-store";
import { PierreFile } from "../pierre/pierre-file";

export const isPierreHostReady = (
  selectedPath: string | null,
  fileContent: { path: string } | null,
  session: { path: string } | null
): boolean =>
  selectedPath !== null &&
  fileContent !== null &&
  session !== null &&
  fileContent.path === selectedPath &&
  session.path === fileContent.path;

export const FileViewer = () => {
  const fileContent = useStore(explorerStore, (s) => s.fileContent);
  const selectedPath = useStore(explorerStore, (s) => s.selectedPath);
  const isFileDirty = useStore(explorerStore, (s) => s.isFileDirty);
  const revealLine = useStore(explorerStore, (s) => s.revealLine);
  const colorScheme = useColorScheme();
  const [session, setSession] = createSignal<SaveSession | null>(null);
  const [cacheGeneration, setCacheGeneration] = createSignal(0);

  createEffect(
    () => fileContent()?.path,
    (path) => {
      const content = explorerStore.getState().fileContent;
      if (!path || !content) {
        setSession(null);
        setCacheGeneration(0);
        return;
      }
      const nextSession: SaveSession = { path, diskSha: "", pendingSha: null, cacheGeneration: 0 };
      setSession(nextSession);
      setCacheGeneration(0);
      explorerStore.getState().setIsFileDirty(false);
      void sha256Utf8(content.content).then((sha) => {
        if (session() === nextSession && nextSession.cacheGeneration === 0 && nextSession.diskSha === "") {
          nextSession.diskSha = sha;
        }
      });
    }
  );

  useDiskGuard({
    getSession: session,
    onReload: (content, incomingSha) => {
      const currentSession = session();
      if (!currentSession || currentSession.path !== content.path) return;
      currentSession.diskSha = incomingSha;
      currentSession.pendingSha = null;
      currentSession.cacheGeneration += 1;
      setCacheGeneration(currentSession.cacheGeneration);
      explorerStore.getState().setFileContent(content);
      explorerStore.getState().setIsFileDirty(false);
    },
  });

  const hostCacheKey = createMemo(() => {
    cacheGeneration();
    const currentSession = session();
    if (!currentSession) return "";
    return sessionCacheKey(currentSession);
  });

  const loadedContent = createMemo(() => {
    const content = fileContent();
    const selected = selectedPath();
    if (!content || !selected || content.path !== selected) return null;
    return content;
  });

  const handleOpenInEditor = async () => {
    const path = selectedPath();
    if (!path) return;
    try {
      await commands.openInEditor(path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const handleRevealInFinder = async () => {
    const path = selectedPath();
    if (!path) return;
    try {
      await commands.revealInFileManager(path);
    } catch (err) {
      repositoryStore.getState().setError(String(err));
    }
  };

  const breadcrumbs = createMemo(() => selectedPath()?.split("/") ?? []);
  const fileName = createMemo(() => {
    const path = selectedPath();
    return path ? (path.split("/").pop() ?? path) : "";
  });

  const headerActions = (includeReveal: boolean) => (
    <div class="diff-header-actions">
      {includeReveal && (
        <button class="scm-toolbar-button" onClick={handleRevealInFinder} title="Reveal in Finder">
          <span class="codicon codicon-folder-opened" />
        </button>
      )}
      <button class="scm-toolbar-button" onClick={handleOpenInEditor} title="Open in Editor">
        <span class="codicon codicon-go-to-file" />
      </button>
    </div>
  );

  const breadcrumbTrail = (showDirty: boolean) => (
    <span class="file-viewer-breadcrumbs" title={selectedPath() ?? ""}>
      <For each={breadcrumbs()} keyed={false}>
        {(part, index) => (
          <span>
            {index > 0 && <span class="file-viewer-separator"> / </span>}
            {part()}
          </span>
        )}
      </For>
      {showDirty && isFileDirty() && <span class="dirty-indicator"> *</span>}
    </span>
  );

  return (
    <>
      {!loadedContent() ? (
        <div class="diff-empty-state">
          <img
            class="diff-empty-watermark"
            src={colorScheme() === "dark" ? "/deathpush-white.png" : "/deathpush-black.png"}
            alt=""
          />
          <p style={{ opacity: 0.4, "margin-top": "12px" }}>Select a file to view its contents</p>
        </div>
      ) : loadedContent()!.fileType === "large" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(false)}
          </div>
          <div class="file-viewer-message">
            <span class="codicon codicon-warning" style={{ "font-size": "32px", opacity: 0.4 }} />
            <p>File is too large to display (over 5 MB)</p>
            <button class="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
              Open in External Editor
            </button>
          </div>
        </div>
      ) : loadedContent()!.fileType === "binary" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(false)}
          </div>
          <div class="file-viewer-message">
            <span class="codicon codicon-file-binary" style={{ "font-size": "32px", opacity: 0.4 }} />
            <p>Binary file cannot be displayed</p>
            <button class="action-button" style={{ width: "auto", padding: "0 12px" }} onClick={handleOpenInEditor}>
              Open in External Editor
            </button>
          </div>
        </div>
      ) : loadedContent()!.fileType === "image" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(true)}
          </div>
          <div class="file-viewer-image">
            <img src={loadedContent()!.content} alt={fileName()} />
          </div>
        </div>
      ) : (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(true)}
            {headerActions(true)}
          </div>
          <div class="diff-editor-container">
            {isPierreHostReady(selectedPath(), loadedContent(), session()) && hostCacheKey() && (
              <PierreFile
                path={loadedContent()!.path}
                contents={loadedContent()!.content}
                cacheKey={hostCacheKey()}
                revealLine={revealLine()}
                session={session()!}
              />
            )}
          </div>
        </div>
      )}
    </>
  );
};
