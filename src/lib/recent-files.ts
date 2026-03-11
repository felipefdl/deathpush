export interface RecentFile {
  path: string;
  lastOpened: string;
}

const MAX = 20;

const storageKey = (repoRoot: string): string => `deathpush:recentFiles:${btoa(repoRoot)}`;

export const getRecentFiles = (repoRoot: string): RecentFile[] => {
  try {
    const raw = localStorage.getItem(storageKey(repoRoot));
    if (!raw) return [];
    const parsed: RecentFile[] = JSON.parse(raw);
    return parsed.sort((a, b) => b.lastOpened.localeCompare(a.lastOpened)).slice(0, MAX);
  } catch {
    return [];
  }
};

export const addRecentFile = (repoRoot: string, path: string): void => {
  const files = getRecentFiles(repoRoot).filter((f) => f.path !== path);
  files.unshift({ path, lastOpened: new Date().toISOString() });
  localStorage.setItem(storageKey(repoRoot), JSON.stringify(files.slice(0, MAX)));
};

export const clearRecentFiles = (repoRoot: string): void => {
  localStorage.removeItem(storageKey(repoRoot));
};
