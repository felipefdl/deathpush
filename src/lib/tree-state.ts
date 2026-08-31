export type TreeStateItem = {
  isDirectory(): boolean;
  isExpanded(): boolean;
  expand(): void;
  select(): void;
};

export type TreeStateModel = {
  getItem(path: string): TreeStateItem | null;
  getFocusedPath(): string | null;
  getSelectedPaths(): readonly string[];
  focusPath(path: string): void;
};

export const directoryPathCandidates = (path: string): string[] =>
  path.endsWith("/") ? [path, path.slice(0, -1)] : [path, `${path}/`];

export const ancestorDirectoryPaths = (filePath: string): string[] => {
  const normalized = filePath.endsWith("/") ? filePath.slice(0, -1) : filePath;
  const segments = normalized.split("/").filter(Boolean);
  const directories: string[] = [];
  for (let index = 1; index < segments.length; index += 1) {
    directories.push(`${segments.slice(0, index).join("/")}/`);
  }
  return directories;
};

const directoryHandle = (model: TreeStateModel, path: string): TreeStateItem | null => {
  for (const candidate of directoryPathCandidates(path)) {
    const item = model.getItem(candidate);
    if (item?.isDirectory() === true) return item;
  }
  return null;
};

export const snapshotExpandedDirectoryPaths = (model: TreeStateModel, filePaths: readonly string[]): string[] => {
  const directories = new Set<string>();
  for (const path of filePaths) {
    for (const directory of ancestorDirectoryPaths(path)) directories.add(directory);
    if (path.endsWith("/")) directories.add(path);
  }
  return [...directories].filter((path) => {
    const item = directoryHandle(model, path);
    return item?.isExpanded() === true;
  });
};

export const restoreExpandedDirectoryPaths = (model: TreeStateModel, expandedPaths: readonly string[]): void => {
  for (const path of expandedPaths) directoryHandle(model, path)?.expand();
};

export const nextPersistedExpandedPaths = (current: readonly string[], snapshot: readonly string[]): string[] =>
  snapshot.length === 0 && current.length > 0 ? [...current] : [...snapshot];

export const restoreSelectedFilePath = (model: TreeStateModel, path: string | null): void => {
  if (!path) return;
  const item = model.getItem(path);
  if (!item || item.isDirectory()) return;
  item.select();
  model.focusPath(path);
};
