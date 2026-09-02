import type { DiffPayload, ResourceGroupKind } from "../git-types";

export type ScmDiffHandoff = {
  path: string;
  staged: boolean;
  groupKind: ResourceGroupKind;
  loadId: number;
};

type CachedScmDiff = ScmDiffHandoff & { payload: DiffPayload };

let cached: CachedScmDiff | null = null;

export const rememberScmDiffPayload = (handoff: ScmDiffHandoff, payload: DiffPayload): void => {
  cached = { ...handoff, payload };
};

export const takeScmDiffPayload = (handoff: ScmDiffHandoff): DiffPayload | null => {
  const hit = cached;
  if (
    hit === null ||
    hit.path !== handoff.path ||
    hit.staged !== handoff.staged ||
    hit.groupKind !== handoff.groupKind ||
    hit.loadId !== handoff.loadId
  ) {
    return null;
  }
  cached = null;
  return hit.payload;
};

export const clearScmDiffPayload = (): void => {
  cached = null;
};
