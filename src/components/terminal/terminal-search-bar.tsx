import { useCallback, useEffect, useRef, useState } from "react";
import type { SearchAddon } from "@xterm/addon-search";

interface TerminalSearchBarProps {
  searchAddon: SearchAddon;
  onClose: () => void;
}

export const TerminalSearchBar = ({ searchAddon, onClose }: TerminalSearchBarProps) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const [query, setQuery] = useState("");
  const [resultIndex, setResultIndex] = useState(-1);
  const [resultCount, setResultCount] = useState(0);

  useEffect(() => {
    inputRef.current?.focus();
    const disposable = searchAddon.onDidChangeResults?.((e) => {
      setResultIndex(e.resultIndex);
      setResultCount(e.resultCount);
    });
    return () => disposable?.dispose();
  }, [searchAddon]);

  const handleChange = useCallback(
    (value: string) => {
      setQuery(value);
      if (value) {
        searchAddon.findNext(value, { incremental: true });
      } else {
        searchAddon.clearDecorations();
        setResultIndex(-1);
        setResultCount(0);
      }
    },
    [searchAddon],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        searchAddon.clearDecorations();
        onClose();
      } else if (e.key === "Enter") {
        e.preventDefault();
        if (e.shiftKey) {
          searchAddon.findPrevious(query);
        } else {
          searchAddon.findNext(query);
        }
      }
    },
    [searchAddon, query, onClose],
  );

  const countLabel = query && resultCount >= 0 ? `${resultIndex + 1}/${resultCount}` : "";

  return (
    <div className="terminal-search-bar">
      <input
        ref={inputRef}
        className="terminal-search-input"
        type="text"
        value={query}
        placeholder="Find"
        onChange={(e) => handleChange(e.target.value)}
        onKeyDown={handleKeyDown}
      />
      {countLabel && <span className="terminal-search-count">{countLabel}</span>}
      <button
        className="terminal-search-btn"
        onClick={() => searchAddon.findPrevious(query)}
        title="Previous Match (Shift+Enter)"
      >
        <span className="codicon codicon-chevron-up" />
      </button>
      <button
        className="terminal-search-btn"
        onClick={() => searchAddon.findNext(query)}
        title="Next Match (Enter)"
      >
        <span className="codicon codicon-chevron-down" />
      </button>
      <button
        className="terminal-search-btn"
        onClick={() => {
          searchAddon.clearDecorations();
          onClose();
        }}
        title="Close (Escape)"
      >
        <span className="codicon codicon-close" />
      </button>
    </div>
  );
};
