import { createContext, createEffect, createMemo, For, onSettled } from "solid-js";
import type { GitDecorationMaps } from "../../hooks/use-explorer-git-status";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { ExplorerItem } from "./explorer-item";
import * as commands from "../../lib/tauri-commands";

export const GitDecorationContext = createContext<() => GitDecorationMaps>();

type ExplorerTreeProps = {
  path: string | null;
  depth: number;
  filter: string;
};

type CreateRowProps = {
  parentPath: string | null;
  type: "file" | "folder";
  depth: number;
};

const entryMatchesFilter = (
  entry: { name: string; path: string; isDirectory: boolean },
  filter: string,
  cache: Map<string, { name: string; path: string; isDirectory: boolean }[]>
): boolean => {
  if (!entry.isDirectory) {
    return entry.name.toLowerCase().includes(filter);
  }
  const children = cache.get(entry.path);
  if (!children) return true;
  return children.some((child) => entryMatchesFilter(child, filter, cache));
};

const CreateRow = (props: CreateRowProps) => {
  let inputRef: HTMLInputElement | undefined;

  onSettled(() => {
    inputRef?.focus();
  });

  const handleSubmit = async (name: string) => {
    const { setCreatingIn, clearCache } = explorerStore.getState();
    const { setError } = repositoryStore.getState();
    setCreatingIn(null);
    const trimmed = name.trim();
    if (!trimmed) return;
    const fullPath = props.parentPath ? `${props.parentPath}/${trimmed}` : trimmed;
    try {
      if (props.type === "folder") {
        await commands.createDirectory(fullPath);
      } else {
        await commands.writeFile(fullPath, "");
      }
      clearCache();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleKeyDown = (e: KeyboardEvent & { currentTarget: HTMLInputElement }) => {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleSubmit(e.currentTarget.value);
    } else if (e.key === "Escape") {
      e.preventDefault();
      explorerStore.getState().setCreatingIn(null);
    }
  };

  const iconClass = () => (props.type === "folder" ? "codicon codicon-folder" : "codicon codicon-file");

  return (
    <div class="explorer-create-row" style={{ "padding-left": `${12 + props.depth * 12}px` }}>
      <span class={iconClass()} style={{ "margin-right": "4px", "font-size": "14px" }} />
      <input
        ref={(el) => {
          inputRef = el;
        }}
        class="explorer-item-rename-input"
        placeholder={props.type === "folder" ? "Folder name" : "File name"}
        onKeyDown={handleKeyDown}
        onBlur={(e) => handleSubmit(e.currentTarget.value)}
        autocomplete="off"
        spellcheck={false}
      />
    </div>
  );
};

const ExplorerTreeLevel = (props: ExplorerTreeProps) => {
  const directoryCache = useStore(explorerStore, (s) => s.directoryCache);
  const expandedDirs = useStore(explorerStore, (s) => s.expandedDirs);
  const creatingIn = useStore(explorerStore, (s) => s.creatingIn);

  const cacheKey = createMemo(() => props.path ?? "__root__");
  const entries = createMemo(() => directoryCache().get(cacheKey()));

  createEffect(
    () => [props.path, cacheKey(), entries()] as const,
    ([path, key, current]) => {
      if (!current) {
        commands
          .listDirectory(path)
          .then((result) => {
            explorerStore.getState().setDirectoryEntries(key, result);
          })
          .catch((err) => repositoryStore.getState().setError(String(err)));
      }
    }
  );

  const handleToggleDir = (dirPath: string) => {
    const { toggleDir, directoryCache: cache, setDirectoryEntries } = explorerStore.getState();
    toggleDir(dirPath);
    if (!cache.has(dirPath)) {
      commands
        .listDirectory(dirPath)
        .then((result) => {
          setDirectoryEntries(dirPath, result);
        })
        .catch((err) => repositoryStore.getState().setError(String(err)));
    }
  };

  const filtered = createMemo(() => {
    const list = entries();
    if (!list || !props.filter) return list ?? [];
    return list.filter((e) => entryMatchesFilter(e, props.filter, directoryCache()));
  });

  const showCreateRow = createMemo(() => {
    const creating = creatingIn();
    return creating !== null && creating.parentPath === props.path;
  });

  const isExpanded = (path: string, isDirectory: boolean): boolean =>
    expandedDirs().has(path) || (!!props.filter && isDirectory);

  return (
    <>
      {entries() ? (
        <>
          {showCreateRow() && creatingIn() && (
            <CreateRow parentPath={creatingIn()!.parentPath} type={creatingIn()!.type} depth={props.depth} />
          )}
          <For each={filtered()} keyed={(entry) => entry.path}>
            {(entry) => {
              const showChildCreate = () => creatingIn()?.parentPath === entry().path;
              return (
                <div>
                  <ExplorerItem
                    entry={entry()}
                    depth={props.depth}
                    onToggleDir={handleToggleDir}
                    expanded={isExpanded(entry().path, entry().isDirectory)}
                  />
                  {entry().isDirectory && (isExpanded(entry().path, entry().isDirectory) || showChildCreate()) && (
                    <ExplorerTreeLevel path={entry().path} depth={props.depth + 1} filter={props.filter} />
                  )}
                </div>
              );
            }}
          </For>
        </>
      ) : null}
    </>
  );
};

export const ExplorerTree = () => {
  const fileFilter = useStore(explorerStore, (s) => s.fileFilter);
  return <ExplorerTreeLevel path={null} depth={0} filter={fileFilter().toLowerCase()} />;
};
