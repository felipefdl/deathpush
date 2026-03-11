import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { getRecentFiles, addRecentFile, clearRecentFiles } from "./recent-files";

const REPO_ROOT = "/home/user/my-repo";
const STORAGE_KEY = `deathpush:recentFiles:${btoa(REPO_ROOT)}`;

describe("recent-files", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-06-15T12:00:00Z"));
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("getRecentFiles", () => {
    it("returns empty array when storage is empty", () => {
      expect(getRecentFiles(REPO_ROOT)).toEqual([]);
    });

    it("returns empty array for invalid JSON", () => {
      localStorage.setItem(STORAGE_KEY, "not-json{{{");
      expect(getRecentFiles(REPO_ROOT)).toEqual([]);
    });

    it("sorts by lastOpened descending", () => {
      localStorage.setItem(
        STORAGE_KEY,
        JSON.stringify([
          { path: "src/a.ts", lastOpened: "2025-06-14T10:00:00Z" },
          { path: "src/c.ts", lastOpened: "2025-06-15T10:00:00Z" },
          { path: "src/b.ts", lastOpened: "2025-06-13T10:00:00Z" },
        ]),
      );
      const result = getRecentFiles(REPO_ROOT);
      expect(result.map((f) => f.path)).toEqual(["src/c.ts", "src/a.ts", "src/b.ts"]);
    });

    it("caps at 20 entries", () => {
      const entries = Array.from({ length: 25 }, (_, i) => ({
        path: `file-${i}.ts`,
        lastOpened: `2025-06-15T${String(i).padStart(2, "0")}:00:00Z`,
      }));
      localStorage.setItem(STORAGE_KEY, JSON.stringify(entries));
      expect(getRecentFiles(REPO_ROOT)).toHaveLength(20);
    });

    it("isolates storage by repo root", () => {
      addRecentFile(REPO_ROOT, "src/a.ts");
      addRecentFile("/other/repo", "src/b.ts");
      expect(getRecentFiles(REPO_ROOT)).toHaveLength(1);
      expect(getRecentFiles(REPO_ROOT)[0].path).toBe("src/a.ts");
      expect(getRecentFiles("/other/repo")).toHaveLength(1);
      expect(getRecentFiles("/other/repo")[0].path).toBe("src/b.ts");
    });
  });

  describe("addRecentFile", () => {
    it("adds a file to an empty list", () => {
      addRecentFile(REPO_ROOT, "src/main.ts");
      const result = getRecentFiles(REPO_ROOT);
      expect(result).toHaveLength(1);
      expect(result[0].path).toBe("src/main.ts");
      expect(result[0].lastOpened).toBe("2025-06-15T12:00:00.000Z");
    });

    it("deduplicates by path, updating timestamp", () => {
      addRecentFile(REPO_ROOT, "src/main.ts");
      vi.setSystemTime(new Date("2025-06-15T13:00:00Z"));
      addRecentFile(REPO_ROOT, "src/main.ts");
      const result = getRecentFiles(REPO_ROOT);
      expect(result).toHaveLength(1);
      expect(result[0].lastOpened).toBe("2025-06-15T13:00:00.000Z");
    });

    it("places most recent file first", () => {
      addRecentFile(REPO_ROOT, "src/first.ts");
      vi.setSystemTime(new Date("2025-06-15T13:00:00Z"));
      addRecentFile(REPO_ROOT, "src/second.ts");
      const result = getRecentFiles(REPO_ROOT);
      expect(result[0].path).toBe("src/second.ts");
      expect(result[1].path).toBe("src/first.ts");
    });

    it("caps at 20 files", () => {
      for (let i = 0; i < 21; i++) {
        vi.setSystemTime(new Date(`2025-06-15T${String(i).padStart(2, "0")}:00:00Z`));
        addRecentFile(REPO_ROOT, `file-${i}.ts`);
      }
      const result = getRecentFiles(REPO_ROOT);
      expect(result).toHaveLength(20);
    });

    it("evicts oldest file when exceeding cap", () => {
      for (let i = 0; i < 20; i++) {
        vi.setSystemTime(new Date(`2025-06-15T${String(i).padStart(2, "0")}:00:00Z`));
        addRecentFile(REPO_ROOT, `file-${i}.ts`);
      }
      vi.setSystemTime(new Date("2025-06-15T20:00:00Z"));
      addRecentFile(REPO_ROOT, "file-new.ts");

      const result = getRecentFiles(REPO_ROOT);
      expect(result).toHaveLength(20);
      expect(result[0].path).toBe("file-new.ts");
      expect(result.find((f) => f.path === "file-0.ts")).toBeUndefined();
    });

    it("re-adding existing file moves it to the top", () => {
      addRecentFile(REPO_ROOT, "src/a.ts");
      vi.setSystemTime(new Date("2025-06-15T13:00:00Z"));
      addRecentFile(REPO_ROOT, "src/b.ts");
      vi.setSystemTime(new Date("2025-06-15T14:00:00Z"));
      addRecentFile(REPO_ROOT, "src/c.ts");

      // Re-add "a" which was oldest
      vi.setSystemTime(new Date("2025-06-15T15:00:00Z"));
      addRecentFile(REPO_ROOT, "src/a.ts");

      const result = getRecentFiles(REPO_ROOT);
      expect(result.map((f) => f.path)).toEqual(["src/a.ts", "src/c.ts", "src/b.ts"]);
    });
  });

  describe("clearRecentFiles", () => {
    it("clears all files for a repo", () => {
      addRecentFile(REPO_ROOT, "src/a.ts");
      addRecentFile(REPO_ROOT, "src/b.ts");
      clearRecentFiles(REPO_ROOT);
      expect(getRecentFiles(REPO_ROOT)).toEqual([]);
    });

    it("is a no-op when already empty", () => {
      clearRecentFiles(REPO_ROOT);
      expect(getRecentFiles(REPO_ROOT)).toEqual([]);
    });

    it("does not affect other repos", () => {
      addRecentFile(REPO_ROOT, "src/a.ts");
      addRecentFile("/other/repo", "src/b.ts");
      clearRecentFiles(REPO_ROOT);
      expect(getRecentFiles(REPO_ROOT)).toEqual([]);
      expect(getRecentFiles("/other/repo")).toHaveLength(1);
    });
  });
});
