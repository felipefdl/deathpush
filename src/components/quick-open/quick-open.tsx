import { useState, useEffect, useRef, useCallback } from "react";
import { useExplorerStore } from "../../stores/explorer-store";
import { useLayoutStore } from "../../stores/layout-store";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";
import * as commands from "../../lib/tauri-commands";
import type { FuzzyFileResult, ContentSearchResult } from "../../lib/git-types";

interface QuickOpenProps {
  onClose: () => void;
}

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

  // Load initial file list
  useEffect(() => {
    commands.fuzzyFindFiles("", 100).then(setFileResults).catch(() => {});
    inputRef.current?.focus();
  }, []);

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
      const timer = setTimeout(() => {
        commands
          .fuzzyFindFiles(fileQuery, 100)
          .then((results) => {
            setFileResults(results);
            setActiveIndex(0);
          })
          .catch(() => setFileResults([]));
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
        })
        .catch(() => {});
      layout.setSidebarView("explorer");
      layout.setMainView("file");
      onClose();
    },
    [onClose],
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

  const totalItems = isContentMode ? contentResults.length : fileResults.length;

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
          if (activeIndex >= 0 && activeIndex < fileResults.length) {
            selectFile(fileResults[activeIndex].path, goToLine);
          }
        }
      } else if (e.key === "Escape") {
        e.preventDefault();
        onClose();
      }
    },
    [totalItems, activeIndex, isContentMode, isGoToLineOnly, contentResults, fileResults, selectFile, goToCurrentFileLine, onClose, goToLine],
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
                  <span className="quick-open-item-content">{result.lineContent.trim()}</span>
                </div>
              ))
            ) : (
              <div className="quick-open-empty">
                {loading ? "Searching..." : search.length > 1 ? "No results" : "Type to search file contents"}
              </div>
            )
          ) : fileResults.length > 0 ? (
            fileResults.map((result, i) => {
              const fileName = getFileName(result.path);
              const dirPath = getDirPath(result.path);
              // Map match positions to filename-relative indices
              const nameStart = result.path.length - fileName.length;
              const namePositions = new Set(
                result.matchPositions.filter((p) => p >= nameStart).map((p) => p - nameStart),
              );
              return (
                <div
                  key={result.path}
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
