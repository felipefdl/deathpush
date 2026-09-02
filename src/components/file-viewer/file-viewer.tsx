import { createEffect, createMemo, createSignal, For } from "solid-js";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { useColorScheme } from "../../hooks/use-color-scheme";
import { useDiskGuard } from "../../hooks/use-disk-guard";
import type { FileContent } from "../../lib/git-types";
import * as commands from "../../lib/tauri-commands";
import { sessionCacheKey, type SaveSession } from "../../lib/pierre/save-session";
import { useStore } from "../../lib/use-store";
import { PierreFile } from "../pierre/pierre-file";

type DisplayedFile = {
  content: FileContent;
  session: SaveSession;
  cacheKey: string;
};

const createDisplayedFile = (content: FileContent, session: SaveSession): DisplayedFile => ({
  content,
  session,
  cacheKey: sessionCacheKey(session),
});

export const isPierreHostReady = (fileContent: { path: string } | null, session: { path: string } | null): boolean =>
  fileContent !== null && session !== null && session.path === fileContent.path;

export const FileViewer = () => {
  const fileContent = useStore(explorerStore, (s) => s.fileContent);
  const selectedPath = useStore(explorerStore, (s) => s.selectedPath);
  const isFileDirty = useStore(explorerStore, (s) => s.isFileDirty);
  const revealLine = useStore(explorerStore, (s) => s.revealLine);
  const colorScheme = useColorScheme();
  const [session, setSession] = createSignal<SaveSession | null>(null);
  const [displayed, setDisplayed] = createSignal<DisplayedFile | null>(null);
  let sessionPath: string | null = null;

  createEffect(
    () => fileContent()?.path ?? null,
    (path) => {
      if (path === sessionPath) return;
      sessionPath = path;
      const content = explorerStore.getState().fileContent;
      if (!path || !content) {
        setSession(null);
        return;
      }
      const nextSession: SaveSession = {
        path,
        diskSha: content.contentHash,
        pendingSha: null,
        cacheGeneration: 0,
      };
      setSession(nextSession);
      explorerStore.getState().setIsFileDirty(false);
    }
  );

  createEffect(
    () => [selectedPath(), fileContent(), session()] as const,
    ([selected, content, currentSession]) => {
      if (!selected) {
        setDisplayed(null);
        return;
      }
      if (content && currentSession && isPierreHostReady(content, currentSession) && content.path === selected) {
        setDisplayed(createDisplayedFile(content, currentSession));
      }
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
      setDisplayed(createDisplayedFile(content, currentSession));
      explorerStore.getState().setFileContent(content);
      explorerStore.getState().setIsFileDirty(false);
    },
  });

  const hostCacheKey = createMemo(() => displayed()?.cacheKey ?? "");

  const shownContent = createMemo(() => displayed()?.content ?? null);
  const headerPath = createMemo(() => shownContent()?.path ?? selectedPath());

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

  const breadcrumbs = createMemo(() => headerPath()?.split("/") ?? []);
  const fileName = createMemo(() => {
    const path = headerPath();
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
    <span class="file-viewer-breadcrumbs" title={headerPath() ?? ""}>
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
      {!shownContent() && !selectedPath() ? (
        <div class="diff-empty-state">
          <img
            class="diff-empty-watermark"
            src={colorScheme() === "dark" ? "/deathpush-white.png" : "/deathpush-black.png"}
            alt=""
          />
          <p style={{ opacity: 0.4, "margin-top": "12px" }}>Select a file to view its contents</p>
        </div>
      ) : !shownContent() ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(true)}
          </div>
          <div class="diff-editor-container" />
        </div>
      ) : shownContent()!.fileType === "large" ? (
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
      ) : shownContent()!.fileType === "binary" ? (
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
      ) : shownContent()!.fileType === "image" ? (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(false)}
            {headerActions(true)}
          </div>
          <div class="file-viewer-image">
            <img src={shownContent()!.content} alt={fileName()} />
          </div>
        </div>
      ) : (
        <div class="diff-viewer">
          <div class="file-viewer-header">
            {breadcrumbTrail(true)}
            {headerActions(true)}
          </div>
          <div class="diff-editor-container">
            {hostCacheKey() && (
              <PierreFile
                path={shownContent()!.path}
                contents={shownContent()!.content}
                cacheKey={hostCacheKey()}
                revealLine={revealLine()}
                session={displayed()!.session}
              />
            )}
          </div>
        </div>
      )}
    </>
  );
};
