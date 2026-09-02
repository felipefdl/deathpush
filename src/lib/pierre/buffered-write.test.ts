import { describe, it, expect } from "vite-plus/test";
import { commitPierreWrite } from "./buffered-write";
import type { SaveSession } from "./save-session";

const session = (overrides: Partial<SaveSession> = {}): SaveSession => ({
  path: "src/a.ts",
  diskSha: "aaa",
  pendingSha: null,
  cacheGeneration: 0,
  ...overrides,
});

describe("commitPierreWrite", () => {
  it("rejects and keeps the buffer when writeFile fails", async () => {
    const pending = { text: "edited" };
    const current = session();

    await expect(
      commitPierreWrite({
        writeFile: async () => {
          throw new Error("disk full");
        },
        pending,
        text: "edited",
        session: current,
        sha256Utf8: async () => "bbb",
        syncDirty: () => undefined,
      })
    ).rejects.toThrow("disk full");

    expect(pending.text).toBe("edited");
    expect(current.pendingSha).toBeNull();
    expect(current.diskSha).toBe("aaa");
  });

  it("clears the matching buffer after a successful write", async () => {
    const pending = { text: "edited" };
    const current = session();

    await commitPierreWrite({
      writeFile: async () => ({ contentHash: "bbb" }),
      pending,
      text: "edited",
      session: current,
      sha256Utf8: async () => "bbb",
      syncDirty: () => undefined,
    });

    expect(pending.text).toBeNull();
    expect(current.pendingSha).toBeNull();
    expect(current.diskSha).toBe("bbb");
  });

  it("sets pendingSha to the local hash while write is in flight", async () => {
    const current = session();
    let pendingDuringWrite: string | null = null;
    await commitPierreWrite({
      writeFile: async () => {
        pendingDuringWrite = current.pendingSha;
        return { contentHash: "from-rust" };
      },
      pending: { text: "edited" },
      text: "edited",
      session: current,
      sha256Utf8: async () => "pending-local",
      syncDirty: () => undefined,
    });
    expect(pendingDuringWrite).toBe("pending-local");
  });

  it("sets diskSha from the write result hash", async () => {
    const current = session();
    await commitPierreWrite({
      writeFile: async () => ({ contentHash: "from-rust" }),
      pending: { text: "edited" },
      text: "edited",
      session: current,
      sha256Utf8: async () => "pending-local",
      syncDirty: () => undefined,
    });
    expect(current.pendingSha).toBeNull();
    expect(current.diskSha).toBe("from-rust");
  });
});
