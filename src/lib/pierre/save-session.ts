export type SaveSession = {
  path: string;
  diskSha: string;
  pendingSha: string | null;
  cacheGeneration: number;
};

export const sessionCacheKey = (session: SaveSession): string =>
  session.cacheGeneration === 0 ? session.path : `${session.path}#${session.cacheGeneration}`;

export const isDirty = (state: { pendingTimer: boolean; pendingSha: string | null }): boolean =>
  state.pendingTimer || state.pendingSha !== null;

export const watcherAction = (session: SaveSession, incomingSha: string): "ignore" | "reload" => {
  if (session.pendingSha !== null) return "ignore";
  if (incomingSha === session.diskSha) return "ignore";
  return "reload";
};
