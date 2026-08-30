import type { SaveSession } from "./save-session";

export type PierreWriteBuffer = {
  text: string | null;
};

export const commitPierreWrite = async (input: {
  writeFile: () => Promise<void>;
  pending: PierreWriteBuffer;
  text: string;
  session: SaveSession;
  sha256Utf8: (text: string) => Promise<string>;
  syncDirty: () => void;
}): Promise<void> => {
  input.session.pendingSha = await input.sha256Utf8(input.text);
  input.syncDirty();
  try {
    await input.writeFile();
    input.session.diskSha = input.session.pendingSha;
    input.session.pendingSha = null;
    if (input.pending.text === input.text) input.pending.text = null;
    input.syncDirty();
  } catch (error) {
    input.session.pendingSha = null;
    input.syncDirty();
    throw error;
  }
};
