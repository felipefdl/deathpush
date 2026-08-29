import { describe, it, expect } from "vite-plus/test";
import { isDirty, sessionCacheKey, watcherAction, type SaveSession } from "./save-session";

const base = (): SaveSession => ({
  path: "src/a.ts",
  diskSha: "aaa",
  pendingSha: null,
  cacheGeneration: 0,
});

describe("sessionCacheKey", () => {
  it("is path only at generation 0", () => {
    expect(sessionCacheKey(base())).toBe("src/a.ts");
  });
  it("suffixes generation after disk-won reload", () => {
    expect(sessionCacheKey({ ...base(), cacheGeneration: 2 })).toBe("src/a.ts#2");
  });
});

describe("isDirty", () => {
  it("is true while the timer is pending or a write is in flight", () => {
    expect(isDirty({ pendingTimer: true, pendingSha: null })).toBe(true);
    expect(isDirty({ pendingTimer: false, pendingSha: "bbb" })).toBe(true);
    expect(isDirty({ pendingTimer: false, pendingSha: null })).toBe(false);
  });
});

describe("watcherAction", () => {
  it("ignores while pendingSha is set", () => {
    expect(watcherAction({ ...base(), pendingSha: "bbb" }, "ccc")).toBe("ignore");
  });
  it("ignores when incoming equals diskSha", () => {
    expect(watcherAction(base(), "aaa")).toBe("ignore");
  });
  it("reloads when incoming differs and no write is in flight", () => {
    expect(watcherAction(base(), "ccc")).toBe("reload");
  });
});
