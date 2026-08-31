import type { DiffContent } from "../git-types";
import type { SaveSession } from "./save-session";

export type ScmSessionHandle = {
  session: SaveSession;
  reload: (diff: DiffContent, incomingSha: string) => void;
};

let scmHandle: ScmSessionHandle | null = null;

export const registerScmSession = (handle: ScmSessionHandle): (() => void) => {
  scmHandle = handle;
  return () => {
    if (scmHandle === handle) scmHandle = null;
  };
};

export const getScmSession = (): ScmSessionHandle | null => scmHandle;
