import { describe, it, expect, vi, beforeEach, afterEach } from "vitest";
import { getRecentProjects, addRecentProject, removeRecentProject, clearRecentProjects } from "./recent-projects";

describe("recent-projects", () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date("2025-06-15T12:00:00Z"));
    localStorage.clear();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  describe("getRecentProjects", () => {
    it("returns empty array when storage is empty", () => {
      expect(getRecentProjects()).toEqual([]);
    });

    it("returns empty array for invalid JSON", () => {
      localStorage.setItem("deathpush:recentProjects", "not-json{{{");
      expect(getRecentProjects()).toEqual([]);
    });

    it("normalizes trailing slashes on paths", () => {
      localStorage.setItem(
        "deathpush:recentProjects",
        JSON.stringify([{ path: "/home/user/project/", name: "project", lastOpened: "2025-06-15T11:00:00Z" }]),
      );
      const result = getRecentProjects();
      expect(result[0].path).toBe("/home/user/project");
    });

    it("extracts name from path", () => {
      localStorage.setItem(
        "deathpush:recentProjects",
        JSON.stringify([{ path: "/home/user/my-app", name: "", lastOpened: "2025-06-15T11:00:00Z" }]),
      );
      const result = getRecentProjects();
      expect(result[0].name).toBe("my-app");
    });

    it("sorts by lastOpened descending", () => {
      localStorage.setItem(
        "deathpush:recentProjects",
        JSON.stringify([
          { path: "/a", name: "a", lastOpened: "2025-06-14T10:00:00Z" },
          { path: "/c", name: "c", lastOpened: "2025-06-15T10:00:00Z" },
          { path: "/b", name: "b", lastOpened: "2025-06-13T10:00:00Z" },
        ]),
      );
      const result = getRecentProjects();
      expect(result.map((p) => p.path)).toEqual(["/c", "/a", "/b"]);
    });
  });

  describe("addRecentProject", () => {
    it("adds a project to an empty list", () => {
      addRecentProject("/home/user/project");
      const result = getRecentProjects();
      expect(result).toHaveLength(1);
      expect(result[0].path).toBe("/home/user/project");
      expect(result[0].name).toBe("project");
      expect(result[0].lastOpened).toBe("2025-06-15T12:00:00.000Z");
    });

    it("deduplicates normalized paths", () => {
      addRecentProject("/home/user/project");
      vi.setSystemTime(new Date("2025-06-15T13:00:00Z"));
      addRecentProject("/home/user/project/");
      const result = getRecentProjects();
      expect(result).toHaveLength(1);
      expect(result[0].lastOpened).toBe("2025-06-15T13:00:00.000Z");
    });

    it("caps at 20 projects", () => {
      for (let i = 0; i < 21; i++) {
        vi.setSystemTime(new Date(`2025-06-15T${String(i).padStart(2, "0")}:00:00Z`));
        addRecentProject(`/project/${i}`);
      }
      const result = getRecentProjects();
      expect(result).toHaveLength(20);
    });

    it("places most recent project first", () => {
      addRecentProject("/first");
      vi.setSystemTime(new Date("2025-06-15T13:00:00Z"));
      addRecentProject("/second");
      const result = getRecentProjects();
      expect(result[0].path).toBe("/second");
    });

    it("handles double trailing slashes", () => {
      addRecentProject("/home/user/project//");
      const result = getRecentProjects();
      expect(result[0].path).toBe("/home/user/project");
    });
  });

  describe("removeRecentProject", () => {
    it("removes a project by path", () => {
      addRecentProject("/home/user/a");
      addRecentProject("/home/user/b");
      removeRecentProject("/home/user/a");
      const result = getRecentProjects();
      expect(result).toHaveLength(1);
      expect(result[0].path).toBe("/home/user/b");
    });

    it("is a no-op for unknown path", () => {
      addRecentProject("/home/user/a");
      removeRecentProject("/home/user/unknown");
      expect(getRecentProjects()).toHaveLength(1);
    });
  });

  describe("clearRecentProjects", () => {
    it("clears all projects", () => {
      addRecentProject("/a");
      addRecentProject("/b");
      clearRecentProjects();
      expect(getRecentProjects()).toEqual([]);
    });

    it("is a no-op when already empty", () => {
      clearRecentProjects();
      expect(getRecentProjects()).toEqual([]);
    });
  });
});
