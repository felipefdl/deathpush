import { describe, it, expect } from "vitest";
import { buildWorkspaceTree } from "./workspace-tree";
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
