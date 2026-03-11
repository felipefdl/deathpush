import React, { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useExplorerStore } from "../../stores/explorer-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import { getRecentFiles, addRecentFile } from "../../lib/recent-files";
import * as commands from "../../lib/tauri-commands";
import type { FuzzyFileResult, ContentSearchResult } from "../../lib/git-types";

interface QuickOpenProps {
  onClose: () => void;
}

const HighlightedContent = ({ text, query }: { text: string; query: string }) => {
  if (!query) return <>{text}</>;
  const lowerText = text.toLowerCase();
  const lowerQuery = query.toLowerCase();
  const parts: React.ReactNode[] = [];
  let lastIndex = 0;
  let idx = lowerText.indexOf(lowerQuery, lastIndex);
  while (idx !== -1) {
    if (idx > lastIndex) parts.push(text.slice(lastIndex, idx));
    parts.push(
      <span key={idx} className="quick-open-highlight">
        {text.slice(idx, idx + query.length)}
      </span>,
    );
    lastIndex = idx + query.length;
    idx = lowerText.indexOf(lowerQuery, lastIndex);
  }
  if (lastIndex < text.length) parts.push(text.slice(lastIndex));
  return <>{parts}</>;
};

const HighlightedName = ({ name, positions }: { name: string; positions: Set<number> }) => {
  const parts: React.ReactNode[] = [];
  for (let i = 0; i < name.length; i++) {
    if (positions.has(i)) {
      parts.push(
        <span key={i} className="quick-open-highlight">
          {name[i]}
        </span>,
      );
    } else {
      parts.push(name[i]);
    }
  }
  return <>{parts}</>;
};

export const QuickOpen = ({ onClose }: QuickOpenProps) => {
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState(0);
  const [fileResults, setFileResults] = useState<FuzzyFileResult[]>([]);
  const [contentResults, setContentResults] = useState<ContentSearchResult[]>([]);
  const [loading, setLoading] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);
  const listRef = useRef<HTMLDivElement>(null);
  const isKeyboardNavRef = useRef(false);

  const repoRoot = useRepositoryStore((s) => s.status?.root) ?? "";
  const [recentPaths, setRecentPaths] = useState<string[]>([]);

  const isContentMode = search.startsWith("#");
  const isGoToLineOnly = /^:(\d+)$/.test(search);

  // Parse "filename:line" or ":line" syntax for go-to-line
  const { fileQuery, goToLine } = (() => {
    if (isContentMode) return { fileQuery: search, goToLine: undefined };
    if (isGoToLineOnly) {
      return { fileQuery: "", goToLine: parseInt(search.slice(1), 10) };
    }
    const colonMatch = search.match(/^(.+?):(\d+)$/);
    if (colonMatch) {
      return { fileQuery: colonMatch[1], goToLine: parseInt(colonMatch[2], 10) };
    }
    return { fileQuery: search, goToLine: undefined };
  })();

  // Load initial file list and recent files
  useEffect(() => {
    setLoading(true);
    commands
      .fuzzyFindFiles("", 100)
      .then(setFileResults)
      .catch(() => {})
      .finally(() => setLoading(false));
    if (repoRoot) {
      setRecentPaths(getRecentFiles(repoRoot).map((f) => f.path));
    }
    inputRef.current?.focus();
  }, [repoRoot]);

  // Debounced search
  useEffect(() => {
    if (isContentMode) {
      const query = search.slice(1);
      if (!query) {
        setContentResults([]);
        return;
      }
      setLoading(true);
      const timer = setTimeout(() => {
        commands
          .searchFileContents(query, 100)
          .then((results) => {
            setContentResults(results);
            setActiveIndex(0);
          })
          .catch(() => setContentResults([]))
          .finally(() => setLoading(false));
      }, 300);
      return () => clearTimeout(timer);
    } else {
      setLoading(true);
      const timer = setTimeout(() => {
        commands
          .fuzzyFindFiles(fileQuery, 100)
          .then((results) => {
            setFileResults(results);
            setActiveIndex(0);
          })
          .catch(() => setFileResults([]))
          .finally(() => setLoading(false));
      }, 100);
      return () => clearTimeout(timer);
    }
  }, [search, isContentMode, fileQuery]);

  // Scroll active item into view
  useEffect(() => {
    if (activeIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll("[data-quick-open-item]");
    items[activeIndex]?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const selectFile = useCallback(
    (path: string, lineNumber?: number) => {
      const explorer = useExplorerStore.getState();
      const layout = useLayoutStore.getState();
      explorer.setSelectedPath(path);
      if (lineNumber) {
        explorer.setRevealLine(lineNumber);
      }
      commands
        .readFileContent(path)
        .then((result) => {
          explorer.setFileContent(result);
          if (repoRoot) addRecentFile(repoRoot, path);
        })
        .catch(() => {});
      layout.setSidebarView("explorer");
      layout.setMainView("file");
      onClose();
    },
    [onClose, repoRoot],
  );

  const goToCurrentFileLine = useCallback(
    (line: number) => {
      const explorer = useExplorerStore.getState();
      const currentPath = explorer.selectedPath;
      if (currentPath && explorer.fileContent) {
        explorer.setRevealLine(line);
        useLayoutStore.getState().setMainView("file");
      }
      onClose();
    },
    [onClose],
  );

  // When search is empty, show recent files first, then the rest
  const { orderedFiles, recentCount } = useMemo(() => {
    if (isContentMode || fileQuery) {
      return { orderedFiles: fileResults, recentCount: 0 };
    }
    const recentSet = new Set(recentPaths);
    const recent: FuzzyFileResult[] = [];
    const rest: FuzzyFileResult[] = [];
    for (const r of fileResults) {
      if (recentSet.has(r.path)) {
        recent.push(r);
      } else {
        rest.push(r);
      }
    }
    // Sort recent by their order in recentPaths
    recent.sort((a, b) => recentPaths.indexOf(a.path) - recentPaths.indexOf(b.path));
    return { orderedFiles: [...recent, ...rest], recentCount: recent.length };
  }, [fileResults, recentPaths, isContentMode, fileQuery]);

  const totalItems = isContentMode ? contentResults.length : orderedFiles.length;

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "ArrowDown" && !isGoToLineOnly) {
        e.preventDefault();
        isKeyboardNavRef.current = true;
        setActiveIndex((prev) => (totalItems > 0 ? (prev + 1) % totalItems : 0));
      } else if (e.key === "ArrowUp" && !isGoToLineOnly) {
        e.preventDefault();
        isKeyboardNavRef.current = true;
        setActiveIndex((prev) => (totalItems > 0 ? (prev - 1 + totalItems) % totalItems : 0));
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (isGoToLineOnly && goToLine) {
          goToCurrentFileLine(goToLine);
        } else if (isContentMode) {
          if (activeIndex >= 0 && activeIndex < contentResults.length) {
            const r = contentResults[activeIndex];
            selectFile(r.path, r.lineNumber);
          }
        } else {
          if (activeIndex >= 0 && activeIndex < orderedFiles.length) {
            selectFile(orderedFiles[activeIndex].path, goToLine);
          }
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [totalItems, activeIndex, isContentMode, isGoToLineOnly, contentResults, orderedFiles, selectFile, goToCurrentFileLine, onClose, goToLine],
  );

  const getFileName = (path: string) => {
    const parts = path.split("/");
    return parts[parts.length - 1];
  };

  const getDirPath = (path: string) => {
    const parts = path.split("/");
    if (parts.length <= 1) return "";
    return parts.slice(0, -1).join("/");
  };

  return (
    <div className="quick-open-overlay" onMouseDown={onClose}>
      <div className="quick-open" onMouseDown={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <input
          ref={inputRef}
          className="quick-open-input"
          type="search"
          placeholder="Search files by name (append : to go to line, # to search content)"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          data-form-type="other"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        {loading && <div className="quick-open-loading-bar" />}
        <div className="quick-open-list" ref={listRef} onMouseMove={() => { isKeyboardNavRef.current = false; }}>
          {isGoToLineOnly ? (
            <div className="quick-open-goto-line">
              {goToLine
                ? <>Go to line <b>{goToLine}</b> in current file. Press Enter to confirm.</>
                : "Type a line number to go to."}
            </div>
          ) : isContentMode ? (
            contentResults.length > 0 ? (
              contentResults.map((result, i) => (
                <div
                  key={`${result.path}:${result.lineNumber}:${i}`}
                  data-quick-open-item
                  className={`quick-open-item${i === activeIndex ? " active" : ""}`}
                  onMouseEnter={() => { if (!isKeyboardNavRef.current) setActiveIndex(i); }}
                  onClick={() => selectFile(result.path, result.lineNumber)}
                >
                  <span className={`quick-open-item-icon ${getFileIconClasses(result.path, "file")}`} />
                  <span className="quick-open-item-name">{getFileName(result.path)}:<b>{result.lineNumber}</b></span>
                  {getDirPath(result.path) && <span className="quick-open-item-path">{getDirPath(result.path)}</span>}
                  <span className="quick-open-item-content">
                    <HighlightedContent text={result.lineContent.trim()} query={search.slice(1)} />
                  </span>
                </div>
              ))
            ) : (
              <div className="quick-open-empty">
                {loading ? "Searching..." : search.length > 1 ? "No results" : "Type to search file contents"}
              </div>
            )
          ) : orderedFiles.length > 0 ? (
            orderedFiles.map((result, i) => {
              const fileName = getFileName(result.path);
              const dirPath = getDirPath(result.path);
              // Map match positions to filename-relative indices
              const nameStart = result.path.length - fileName.length;
              const namePositions = new Set(
                result.matchPositions.filter((p) => p >= nameStart).map((p) => p - nameStart),
              );
              const sectionLabel =
                !fileQuery && recentCount > 0 && (i === 0 ? "recently opened" : i === recentCount ? "files" : null);
              return (
                <React.Fragment key={result.path}>
                  {sectionLabel && <div className="quick-open-section-label">{sectionLabel}</div>}
                  <div
                    data-quick-open-item
                    className={`quick-open-item${i === activeIndex ? " active" : ""}`}
                    onMouseEnter={() => { if (!isKeyboardNavRef.current) setActiveIndex(i); }}
                    onClick={() => selectFile(result.path, goToLine)}
                  >
                    <span className={`quick-open-item-icon ${getFileIconClasses(result.path, "file")}`} />
                    <span className="quick-open-item-name">
                      <HighlightedName name={fileName} positions={namePositions} />
                    </span>
                    {goToLine && <span className="quick-open-item-line">:{goToLine}</span>}
                    {dirPath && <span className="quick-open-item-path">{dirPath}</span>}
                  </div>
                </React.Fragment>
              );
            })
          ) : (
            <div className="quick-open-empty">No matching files</div>
          )}
        </div>
      </div>
    </div>
  );
};
