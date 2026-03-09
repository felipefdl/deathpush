import { useCallback, useEffect, useMemo } from "react";
import { useExplorerStore } from "../../stores/explorer-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { ExplorerItem } from "./explorer-item";
import * as commands from "../../lib/tauri-commands";

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

const ExplorerTreeLevel = ({ path, depth, filter }: ExplorerTreeProps) => {
  const directoryCache = useExplorerStore((s) => s.directoryCache);
  const expandedDirs = useExplorerStore((s) => s.expandedDirs);
  const setDirectoryEntries = useExplorerStore((s) => s.setDirectoryEntries);
  const toggleDir = useExplorerStore((s) => s.toggleDir);
  const setError = useRepositoryStore((s) => s.setError);

  const cacheKey = path ?? "__root__";
  const entries = directoryCache.get(cacheKey);

  useEffect(() => {
    if (!entries) {
      commands.listDirectory(path).then((result) => {
        setDirectoryEntries(cacheKey, result);
      }).catch((err) => setError(String(err)));
    }
  }, [path, cacheKey, entries, setDirectoryEntries, setError]);

  const handleToggleDir = useCallback((dirPath: string) => {
    toggleDir(dirPath);
    if (!directoryCache.has(dirPath)) {
      commands.listDirectory(dirPath).then((result) => {
        setDirectoryEntries(dirPath, result);
      }).catch((err) => setError(String(err)));
    }
  }, [toggleDir, directoryCache, setDirectoryEntries, setError]);

  const filtered = useMemo(() => {
    if (!entries || !filter) return entries ?? [];
    return entries.filter((e) => entryMatchesFilter(e, filter, directoryCache));
  }, [entries, filter, directoryCache]);

  if (!entries) return null;

  return (
    <>
      {filtered.map((entry) => {
        const isExpanded = expandedDirs.has(entry.path) || (!!filter && entry.isDirectory);
        return (
          <div key={entry.path}>
            <ExplorerItem
              entry={entry}
              depth={depth}
              onToggleDir={handleToggleDir}
              expanded={isExpanded}
            />
            {entry.isDirectory && isExpanded && (
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
