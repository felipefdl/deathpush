import { useTauriEvent } from "./use-tauri-event";
import { applySessionStatus } from "../lib/session-client";
import type { PathsChanged, SessionStatusEvent } from "../lib/git-types";

export const pathsChangedIntersects = (event: PathsChanged, target: string | null): boolean => {
  if (target === null) return false;
  if (event.scope === "repository") return true;
  if (event.scope === "exact") return event.paths.includes(target);
  return event.paths.some((path) => target === path || target.startsWith(`${path}/`) || path.startsWith(`${target}/`));
};

export const shouldRefreshExplorer = (event: PathsChanged): boolean =>
  event.scope === "repository" || event.scope === "subtree" || event.kind === "structural";

export const useRepositoryEvents = (): void => {
  useTauriEvent<SessionStatusEvent>("session:status", (event) => {
    applySessionStatus(event);
  });
};
