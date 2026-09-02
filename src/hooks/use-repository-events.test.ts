import { describe, expect, it } from "vite-plus/test";
import type { PathsChanged } from "../lib/git-types";
import { pathsChangedIntersects, shouldRefreshExplorer } from "./use-repository-events";

describe("pathsChangedIntersects", () => {
  const event = (overrides: Partial<PathsChanged>): PathsChanged => ({
    paths: [],
    kind: "content",
    scope: "exact",
    generation: 1,
    storm: false,
    ...overrides,
  });

  it("matches an exact path and repository scope", () => {
    expect(pathsChangedIntersects(event({ paths: ["src/a.ts"], scope: "exact" }), "src/a.ts")).toBe(true);
    expect(pathsChangedIntersects(event({ paths: ["src/a.ts"], scope: "exact" }), "src/b.ts")).toBe(false);
    expect(pathsChangedIntersects(event({ paths: [], scope: "repository" }), "anything.ts")).toBe(true);
  });

  it("matches a subtree path", () => {
    expect(pathsChangedIntersects(event({ paths: ["src"], scope: "subtree" }), "src/a.ts")).toBe(true);
    expect(pathsChangedIntersects(event({ paths: ["src"], scope: "subtree" }), "lib/a.ts")).toBe(false);
  });
});

describe("shouldRefreshExplorer", () => {
  it("refreshes on repository, subtree, or structural events", () => {
    expect(
      shouldRefreshExplorer({ paths: [], kind: "content", scope: "repository", generation: 1, storm: false })
    ).toBe(true);
    expect(
      shouldRefreshExplorer({ paths: ["src"], kind: "content", scope: "subtree", generation: 1, storm: false })
    ).toBe(true);
    expect(
      shouldRefreshExplorer({ paths: ["a.ts"], kind: "structural", scope: "exact", generation: 1, storm: false })
    ).toBe(true);
    expect(
      shouldRefreshExplorer({ paths: ["a.ts"], kind: "content", scope: "exact", generation: 1, storm: false })
    ).toBe(false);
  });
});
