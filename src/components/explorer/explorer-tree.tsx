import { createContext, useCallback, useEffect, useMemo, useRef } from "react";
import type { GitDecorationMaps } from "../../hooks/use-explorer-git-status";
import { useExplorerStore } from "../../stores/explorer-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { ExplorerItem } from "./explorer-item";
import * as commands from "../../lib/tauri-commands";

const EMPTY_MAPS: GitDecorationMaps = {
  fileMap: new Map(),
  dirMap: new Map(),
};

export const GitDecorationContext = createContext<GitDecorationMaps>(EMPTY_MAPS);

interface ExplorerTreeProps {
  path: string | null;
  depth: number;
  filter: string;
}

const entryMatchesFilter = (
  entry: { name: string; path: string; isDirectory: boolean },
  filter: string,
  cache: Map<string, { name: string; path: string; isDirectory: boolean }[]>,
): boolean => {
  if (!entry.isDirectory) {
    return entry.name.toLowerCase().includes(filter);
  }
  const children = cache.get(entry.path);
  if (!children) return true; // not loaded yet, keep visible
  return children.some((child) => entryMatchesFilter(child, filter, cache));
};

const CreateRow = ({ parentPath, type, depth }: { parentPath: string | null; type: "file" | "folder"; depth: number }) => {
  const inputRef = useRef<HTMLInputElement>(null);
  const setCreatingIn = useExplorerStore((s) => s.setCreatingIn);
  const clearCache = useExplorerStore((s) => s.clearCache);
  const setError = useRepositoryStore((s) => s.setError);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  const handleSubmit = useCallback(
    async (name: string) => {
      setCreatingIn(null);
      const trimmed = name.trim();
      if (!trimmed) return;
      const fullPath = parentPath ? `${parentPath}/${trimmed}` : trimmed;
      try {
        if (type === "folder") {
          await commands.createDirectory(fullPath);
        } else {
          await commands.writeFile(fullPath, "");
        }
        clearCache();
      } catch (err) {
        setError(String(err));
      }
    },
    [parentPath, type, setCreatingIn, clearCache, setError],
  );

  const handleKeyDown = useCallback(
    (e: React.KeyboardEvent<HTMLInputElement>) => {
      if (e.key === "Enter") {
        e.preventDefault();
        handleSubmit(e.currentTarget.value);
      } else if (e.key === "Escape") {
        e.preventDefault();
        setCreatingIn(null);
      }
    },
    [handleSubmit, setCreatingIn],
  );

  const iconClass = type === "folder" ? "codicon codicon-folder" : "codicon codicon-file";

  return (
    <div className="explorer-create-row" style={{ paddingLeft: 12 + depth * 12 }}>
      <span className={iconClass} style={{ marginRight: 4, fontSize: 14 }} />
      <input
        ref={inputRef}
        className="explorer-item-rename-input"
        placeholder={type === "folder" ? "Folder name" : "File name"}
        onKeyDown={handleKeyDown}
        onBlur={(e) => handleSubmit(e.currentTarget.value)}
        autoComplete="off"
        spellCheck={false}
      />
    </div>
  );
};

const ExplorerTreeLevel = ({ path, depth, filter }: ExplorerTreeProps) => {
  const directoryCache = useExplorerStore((s) => s.directoryCache);
  const expandedDirs = useExplorerStore((s) => s.expandedDirs);
  const setDirectoryEntries = useExplorerStore((s) => s.setDirectoryEntries);
  const toggleDir = useExplorerStore((s) => s.toggleDir);
  const creatingIn = useExplorerStore((s) => s.creatingIn);
  const setError = useRepositoryStore((s) => s.setError);

  const cacheKey = path ?? "__root__";
  const entries = directoryCache.get(cacheKey);

  useEffect(() => {
    if (!entries) {
      commands
        .listDirectory(path)
        .then((result) => {
          setDirectoryEntries(cacheKey, result);
        })
        .catch((err) => setError(String(err)));
    }
  }, [path, cacheKey, entries, setDirectoryEntries, setError]);

  const handleToggleDir = useCallback(
    (dirPath: string) => {
      toggleDir(dirPath);
      if (!directoryCache.has(dirPath)) {
        commands
          .listDirectory(dirPath)
          .then((result) => {
            setDirectoryEntries(dirPath, result);
          })
          .catch((err) => setError(String(err)));
      }
    },
    [toggleDir, directoryCache, setDirectoryEntries, setError],
  );

  const filtered = useMemo(() => {
    if (!entries || !filter) return entries ?? [];
    return entries.filter((e) => entryMatchesFilter(e, filter, directoryCache));
  }, [entries, filter, directoryCache]);

  if (!entries) return null;

  // Determine if a creation row should appear at this level
  const showCreateRow = creatingIn && creatingIn.parentPath === path;

  return (
    <>
      {showCreateRow && <CreateRow parentPath={creatingIn.parentPath} type={creatingIn.type} depth={depth} />}
      {filtered.map((entry) => {
        const isExpanded = expandedDirs.has(entry.path) || (!!filter && entry.isDirectory);
        const showChildCreate = creatingIn && creatingIn.parentPath === entry.path;
        return (
          <div key={entry.path}>
            <ExplorerItem entry={entry} depth={depth} onToggleDir={handleToggleDir} expanded={isExpanded} />
            {entry.isDirectory && (isExpanded || showChildCreate) && (
              <ExplorerTreeLevel path={entry.path} depth={depth + 1} filter={filter} />
            )}
          </div>
        );
      })}
    </>
  );
};

export const ExplorerTree = () => {
  const filter = useExplorerStore((s) => s.fileFilter).toLowerCase();
  return <ExplorerTreeLevel path={null} depth={0} filter={filter} />;
};
