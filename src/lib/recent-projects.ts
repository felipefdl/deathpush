export interface RecentProject {
  path: string;
  name: string;
  lastOpened: string;
}

const STORAGE_KEY = "deathpush:recentProjects";
const MAX = 20;

const normalizePath = (path: string): string => path.replace(/\/+$/, "");

const nameFromPath = (path: string): string => normalizePath(path).split("/").pop() || path;

export const getRecentProjects = (): RecentProject[] => {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed: RecentProject[] = JSON.parse(raw);
    return parsed
      .map((p) => ({ ...p, path: normalizePath(p.path), name: nameFromPath(p.path) }))
      .sort((a, b) => b.lastOpened.localeCompare(a.lastOpened));
  } catch {
    return [];
  }
};

export const addRecentProject = (path: string): void => {
  const normalized = normalizePath(path);
  const projects = getRecentProjects().filter((p) => p.path !== normalized);
  const name = nameFromPath(normalized);
  projects.unshift({ path: normalized, name, lastOpened: new Date().toISOString() });
  localStorage.setItem(STORAGE_KEY, JSON.stringify(projects.slice(0, MAX)));
};

export const removeRecentProject = (path: string): void => {
  const projects = getRecentProjects().filter((p) => p.path !== path);
  localStorage.setItem(STORAGE_KEY, JSON.stringify(projects));
};

export const clearRecentProjects = (): void => {
  localStorage.removeItem(STORAGE_KEY);
};
