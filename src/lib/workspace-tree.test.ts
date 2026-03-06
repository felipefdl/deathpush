import { describe, it, expect } from "vitest";
import { buildWorkspaceTree, buildMultiRootWorkspaceTree } from "./workspace-tree";
import type { ProjectInfo } from "./tauri-commands";

const makeProject = (path: string): ProjectInfo => ({
  path,
  name: path.split("/").pop()!,
});

describe("buildWorkspaceTree", () => {
  it("returns empty root for empty array", () => {
    const root = buildWorkspaceTree([], "/home/user/projects");
    expect(root.children.size).toBe(0);
    expect(root.projects).toEqual([]);
  });

  it("places depth-1 projects into root projects", () => {
    const projects = [
      makeProject("/home/user/projects/alpha"),
      makeProject("/home/user/projects/beta"),
    ];
    const root = buildWorkspaceTree(projects, "/home/user/projects");
    expect(root.projects).toHaveLength(2);
    expect(root.children.size).toBe(0);
  });

  it("creates folder children for depth-2 projects", () => {
    const projects = [
      makeProject("/home/user/projects/work/repo-a"),
      makeProject("/home/user/projects/work/repo-b"),
    ];
    const root = buildWorkspaceTree(projects, "/home/user/projects");
    expect(root.children.size).toBe(1);
    expect(root.children.has("work")).toBe(true);

    const workFolder = root.children.get("work")!;
    expect(workFolder.projects).toHaveLength(2);
    expect(workFolder.projects[0].name).toBe("repo-a");
    expect(workFolder.projects[1].name).toBe("repo-b");
  });

  it("handles mixed depths: folders and top-level projects coexist", () => {
    const projects = [
      makeProject("/root/projects/standalone"),
      makeProject("/root/projects/group/nested"),
    ];
    const root = buildWorkspaceTree(projects, "/root/projects");
    expect(root.projects).toHaveLength(1);
    expect(root.projects[0].name).toBe("standalone");
    expect(root.children.size).toBe(1);
    expect(root.children.get("group")!.projects).toHaveLength(1);
  });

  it("creates nested folders for depth 3+", () => {
    const projects = [
      makeProject("/root/projects/a/b/deep-repo"),
    ];
    const root = buildWorkspaceTree(projects, "/root/projects");
    expect(root.children.has("a")).toBe(true);

    const a = root.children.get("a")!;
    expect(a.children.has("b")).toBe(true);
    expect(a.projects).toHaveLength(0);

    const b = a.children.get("b")!;
    expect(b.projects).toHaveLength(1);
    expect(b.projects[0].name).toBe("deep-repo");
  });

  it("skips projects not under root", () => {
    const projects = [
      makeProject("/home/user/projects/valid"),
      makeProject("/other/path/outside"),
    ];
    const root = buildWorkspaceTree(projects, "/home/user/projects");
    expect(root.projects).toHaveLength(1);
    expect(root.projects[0].name).toBe("valid");
  });

  it("handles root directory with trailing slash", () => {
    const projects = [
      makeProject("/home/user/projects/repo"),
    ];
    const root = buildWorkspaceTree(projects, "/home/user/projects/");
    expect(root.projects).toHaveLength(1);
    expect(root.projects[0].name).toBe("repo");
  });
});

describe("buildMultiRootWorkspaceTree", () => {
  it("returns empty root for empty workspaces", () => {
    const root = buildMultiRootWorkspaceTree([], []);
    expect(root.children.size).toBe(0);
    expect(root.projects).toEqual([]);
  });

  it("creates one child per workspace", () => {
    const projects = [
      makeProject("/home/projects/alpha"),
      makeProject("/home/work/beta"),
    ];
    const workspaces = [
      { directory: "/home/projects", scanDepth: 1 },
      { directory: "/home/work", scanDepth: 1 },
    ];
    const root = buildMultiRootWorkspaceTree(projects, workspaces);
    expect(root.children.size).toBe(2);

    const projNode = root.children.get("/home/projects")!;
    expect(projNode.name).toBe("projects");
    expect(projNode.projects).toHaveLength(1);
    expect(projNode.projects[0].name).toBe("alpha");

    const workNode = root.children.get("/home/work")!;
    expect(workNode.name).toBe("work");
    expect(workNode.projects).toHaveLength(1);
    expect(workNode.projects[0].name).toBe("beta");
  });

  it("builds sub-tree for scanDepth > 1", () => {
    const projects = [
      makeProject("/home/projects/group/repo-a"),
      makeProject("/home/projects/group/repo-b"),
    ];
    const workspaces = [{ directory: "/home/projects", scanDepth: 2 }];
    const root = buildMultiRootWorkspaceTree(projects, workspaces);
    const wsNode = root.children.get("/home/projects")!;
    expect(wsNode.children.has("group")).toBe(true);
    expect(wsNode.children.get("group")!.projects).toHaveLength(2);
  });

  it("assigns projects to the most specific workspace when paths overlap", () => {
    const projects = [
      makeProject("/home/projects/alpha"),
      makeProject("/home/projects/tago/beta"),
      makeProject("/home/projects/tago/gamma"),
    ];
    const workspaces = [
      { directory: "/home/projects", scanDepth: 1 },
      { directory: "/home/projects/tago", scanDepth: 1 },
    ];
    const root = buildMultiRootWorkspaceTree(projects, workspaces);

    const projNode = root.children.get("/home/projects")!;
    expect(projNode.projects).toHaveLength(1);
    expect(projNode.projects[0].name).toBe("alpha");

    const tagoNode = root.children.get("/home/projects/tago")!;
    expect(tagoNode.projects).toHaveLength(2);
  });

  it("deduplicates projects across overlapping workspaces", () => {
    const projects = [
      makeProject("/home/projects/repo"),
      makeProject("/home/projects/repo"),
    ];
    const workspaces = [
      { directory: "/home/projects", scanDepth: 1 },
    ];
    const root = buildMultiRootWorkspaceTree(projects, workspaces);
    const wsNode = root.children.get("/home/projects")!;
    expect(wsNode.projects).toHaveLength(2);
  });
});
