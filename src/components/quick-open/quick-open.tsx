import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import type { JSX } from "@solidjs/web";
import { explorerStore } from "../../stores/explorer-store";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import { getRecentFiles, addRecentFile } from "../../lib/recent-files";
import * as commands from "../../lib/tauri-commands";
import type { FuzzyFileResult, ContentSearchResult } from "../../lib/git-types";

type QuickOpenProps = {
  onClose: () => void;
};

const highlightContent = (text: string, query: string): JSX.Element => {
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const parts: JSX.Element[] = [];
  let lastIndex = 0;
  let idx = lowerText.indexOf(lowerQuery, lastIndex);
  while (idx !== -1) {
    if (idx > lastIndex) parts.push(text.slice(lastIndex, idx));
    parts.push(<span class="quick-open-highlight">{text.slice(idx, idx + query.length)}</span>);
    lastIndex = idx + query.length;
    idx = lowerText.indexOf(lowerQuery, lastIndex);
  }
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return parts;
};

const highlightName = (name: string, positions: Set<number>): JSX.Element => {
  const parts: JSX.Element[] = [];
  for (let i = 0; i < name.length; i++) {
    if (positions.has(i)) {
      parts.push(<span class="quick-open-highlight">{name[i]}</span>);
    } else {
      parts.push(name[i]);
    }
  }
  return parts;
};

const HighlightedContent = (props: { text: string; query: string }) => {
  return <>{props.query ? highlightContent(props.text, props.query) : props.text}</>;
};

const HighlightedName = (props: { name: string; positions: Set<number> }) => {
  return <>{highlightName(props.name, props.positions)}</>;
};

const getFileName = (path: string): string => {
  const parts = path.split("/");
  return parts[parts.length - 1];
};

const getDirPath = (path: string): string => {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  return parts.slice(0, -1).join("/");
};

export const QuickOpen = (props: QuickOpenProps) => {
  const [search, setSearch] = createSignal("");
  const [activeIndex, setActiveIndex] = createSignal(0);
  const [fileResults, setFileResults] = createSignal<FuzzyFileResult[]>([]);
  const [contentResults, setContentResults] = createSignal<ContentSearchResult[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [recentPaths, setRecentPaths] = createSignal<string[]>([]);
  let inputRef: HTMLInputElement | undefined;
  let listRef: HTMLDivElement | undefined;
  let isKeyboardNav = false;

  const repoRoot = useStore(repositoryStore, (s) => s.status?.root ?? "");

  const isContentMode = createMemo(() => search().startsWith("#"));
  const isGoToLineOnly = createMemo(() => /^:(\d+)$/.test(search()));

  const parsedQuery = createMemo(() => {
    const term = search();
    if (term.startsWith("#")) return { fileQuery: term, goToLine: undefined as number | undefined };
    if (/^:(\d+)$/.test(term)) return { fileQuery: "", goToLine: parseInt(term.slice(1), 10) };
    const colonMatch = term.match(/^(.+?):(\d+)$/);
    if (colonMatch) return { fileQuery: colonMatch[1], goToLine: parseInt(colonMatch[2], 10) };
    return { fileQuery: term, goToLine: undefined as number | undefined };
  });

  const fileQuery = createMemo(() => parsedQuery().fileQuery);
  const goToLine = createMemo(() => parsedQuery().goToLine);

  createEffect(
    () => repoRoot(),
    (root) => {
      setLoading(true);
      commands
        .fuzzyFindFiles("", 100)
        .then(setFileResults)
        .catch(() => {})
        .finally(() => setLoading(false));
      if (root) {
        setRecentPaths(getRecentFiles(root).map((f) => f.path));
      }
    }
  );

  createEffect(
    () => [search(), isContentMode(), fileQuery()] as const,
    ([term, contentMode, query]) => {
      if (contentMode) {
        const q = term.slice(1);
        if (!q) {
          setContentResults([]);
          return;
        }
        setLoading(true);
        const timer = setTimeout(() => {
          commands
            .searchFileContents(q, 100)
            .then((results) => {
              setContentResults(results);
              setActiveIndex(0);
            })
            .catch(() => setContentResults([]))
            .finally(() => setLoading(false));
        }, 300);
        return () => clearTimeout(timer);
      }
      setLoading(true);
      const timer = setTimeout(() => {
        commands
          .fuzzyFindFiles(query, 100)
          .then((results) => {
            setFileResults(results);
            setActiveIndex(0);
          })
          .catch(() => setFileResults([]))
          .finally(() => setLoading(false));
      }, 100);
      return () => clearTimeout(timer);
    }
  );

  createEffect(
    () => activeIndex(),
    (idx) => {
      if (idx < 0 || !listRef) return;
      const items = listRef.querySelectorAll("[data-quick-open-item]");
      items[idx]?.scrollIntoView({ block: "nearest" });
    }
  );

  onSettled(() => {
    inputRef?.focus();
  });

  const selectFile = (path: string, lineNumber?: number) => {
    const explorer = explorerStore.getState();
    const layout = layoutStore.getState();
    const root = repositoryStore.getState().status?.root ?? "";
    explorer.setSelectedPath(path);
    if (lineNumber) {
      explorer.setRevealLine(lineNumber);
    }
    commands
      .readFileContent(path)
      .then((result) => {
        explorer.setFileContent(result);
        if (root) addRecentFile(root, path);
      })
      .catch(() => {});
    layout.dockTerminal();
    layout.setSidebarView("explorer");
    layout.setMainView("file");
    props.onClose();
  };

  const goToCurrentFileLine = (line: number) => {
    const explorer = explorerStore.getState();
    const currentPath = explorer.selectedPath;
    if (currentPath && explorer.fileContent) {
      explorer.setRevealLine(line);
      layoutStore.getState().dockTerminal();
      layoutStore.getState().setMainView("file");
    }
    props.onClose();
  };

  const ordered = createMemo(() => {
    if (isContentMode() || fileQuery()) {
      return { orderedFiles: fileResults(), recentCount: 0 };
    }
    const recents = recentPaths();
    const recentSet = new Set(recents);
    const recent: FuzzyFileResult[] = [];
    const rest: FuzzyFileResult[] = [];
    for (const r of fileResults()) {
      if (recentSet.has(r.path)) {
        recent.push(r);
      } else {
        rest.push(r);
      }
    }
    recent.sort((a, b) => recents.indexOf(a.path) - recents.indexOf(b.path));
    return { orderedFiles: [...recent, ...rest], recentCount: recent.length };
  });

  const orderedFiles = createMemo(() => ordered().orderedFiles);
  const recentCount = createMemo(() => ordered().recentCount);
  const totalItems = createMemo(() => (isContentMode() ? contentResults().length : orderedFiles().length));

  const handleKeyDown = (e: KeyboardEvent) => {
    const total = totalItems();
    const lineOnly = isGoToLineOnly();
    const line = goToLine();
    if (e.key === "ArrowDown" && !lineOnly) {
      e.preventDefault();
      isKeyboardNav = true;
      setActiveIndex((prev) => (total > 0 ? (prev + 1) % total : 0));
    } else if (e.key === "ArrowUp" && !lineOnly) {
      e.preventDefault();
      isKeyboardNav = true;
      setActiveIndex((prev) => (total > 0 ? (prev - 1 + total) % total : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (lineOnly && line) {
        goToCurrentFileLine(line);
      } else if (isContentMode()) {
        const results = contentResults();
        const idx = activeIndex();
        if (idx >= 0 && idx < results.length) {
          const r = results[idx];
          selectFile(r.path, r.lineNumber);
        }
      } else {
        const files = orderedFiles();
        const idx = activeIndex();
        if (idx >= 0 && idx < files.length) {
          selectFile(files[idx].path, line);
        }
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      props.onClose();
    }
  };

  return (
    <div class="quick-open-overlay" onMouseDown={() => props.onClose()}>
      <div class="quick-open" onMouseDown={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <input
          ref={(el) => {
            inputRef = el;
          }}
          class="quick-open-input"
          type="search"
          placeholder="Search files by name (append : to go to line, # to search content)"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck={false}
          data-form-type="other"
          value={search()}
          onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => setSearch(e.currentTarget.value)}
        />
        {loading() && <div class="quick-open-loading-bar" />}
        <div
          class="quick-open-list"
          ref={(el) => {
            listRef = el;
          }}
          onMouseMove={() => {
            isKeyboardNav = false;
          }}
        >
          {isGoToLineOnly() ? (
            <div class="quick-open-goto-line">
              {goToLine() ? (
                <>
                  Go to line <b>{goToLine()}</b> in current file. Press Enter to confirm.
                </>
              ) : (
                "Type a line number to go to."
              )}
            </div>
          ) : isContentMode() ? (
            contentResults().length > 0 ? (
              <For
                each={contentResults()}
                keyed={(result) => `${result.path}:${result.lineNumber}:${result.lineContent}`}
              >
                {(result, i) => (
                  <div
                    data-quick-open-item
                    class={["quick-open-item", { active: i() === activeIndex() }]}
                    onMouseEnter={() => {
                      if (!isKeyboardNav) setActiveIndex(i());
                    }}
                    onClick={() => selectFile(result().path, result().lineNumber)}
                  >
                    <span class={["quick-open-item-icon", getFileIconClasses(result().path, "file")]} />
                    <span class="quick-open-item-name">
                      {getFileName(result().path)}:<b>{result().lineNumber}</b>
                    </span>
                    {getDirPath(result().path) && <span class="quick-open-item-path">{getDirPath(result().path)}</span>}
                    <span class="quick-open-item-content">
                      <HighlightedContent text={result().lineContent.trim()} query={search().slice(1)} />
                    </span>
                  </div>
                )}
              </For>
            ) : (
              <div class="quick-open-empty">
                {loading() ? "Searching..." : search().length > 1 ? "No results" : "Type to search file contents"}
              </div>
            )
          ) : orderedFiles().length > 0 ? (
            <For each={orderedFiles()} keyed={(result) => result.path}>
              {(result, i) => {
                const fileName = () => getFileName(result().path);
                const dirPath = () => getDirPath(result().path);
                const namePositions = () => {
                  const name = fileName();
                  const nameStart = result().path.length - name.length;
                  return new Set(
                    result()
                      .matchPositions.filter((p) => p >= nameStart)
                      .map((p) => p - nameStart)
                  );
                };
                const sectionLabel = () => {
                  const idx = i();
                  return !fileQuery() && recentCount() > 0
                    ? idx === 0
                      ? "recently opened"
                      : idx === recentCount()
                        ? "files"
                        : null
                    : null;
                };
                return (
                  <>
                    {sectionLabel() && <div class="quick-open-section-label">{sectionLabel()}</div>}
                    <div
                      data-quick-open-item
                      class={["quick-open-item", { active: i() === activeIndex() }]}
                      onMouseEnter={() => {
                        if (!isKeyboardNav) setActiveIndex(i());
                      }}
                      onClick={() => selectFile(result().path, goToLine())}
                    >
                      <span class={["quick-open-item-icon", getFileIconClasses(result().path, "file")]} />
                      <span class="quick-open-item-name">
                        <HighlightedName name={fileName()} positions={namePositions()} />
                      </span>
                      {goToLine() && <span class="quick-open-item-line">:{goToLine()}</span>}
                      {dirPath() && <span class="quick-open-item-path">{dirPath()}</span>}
                    </div>
                  </>
                );
              }}
            </For>
          ) : (
            <div class="quick-open-empty">No matching files</div>
          )}
        </div>
      </div>
    </div>
  );
};
