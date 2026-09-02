import type { WriteFileResult } from "../git-types";
import type { SaveSession } from "./save-session";

export type PierreWriteBuffer = {
  text: string | null;
};

export const commitPierreWrite = async (input: {
  writeFile: () => Promise<WriteFileResult>;
  pending: PierreWriteBuffer;
  text: string;
  session: SaveSession;
  sha256Utf8: (text: string) => Promise<string>;
  syncDirty: () => void;
}): Promise<void> => {
  input.session.pendingSha = await input.sha256Utf8(input.text);
  input.syncDirty();
  try {
    const result = await input.writeFile();
    input.session.diskSha = result.contentHash;
    input.session.pendingSha = null;
    if (input.pending.text === input.text) input.pending.text = null;
    input.syncDirty();
  } catch (error) {
    input.session.pendingSha = null;
    input.syncDirty();
    throw error;
  }
};
