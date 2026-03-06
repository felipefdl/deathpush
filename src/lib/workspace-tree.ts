import type { ProjectInfo } from "./tauri-commands";
import type { WorkspaceEntry } from "../stores/settings-store";

export interface WorkspaceTreeNode {
  name: string;
  children: Map<string, WorkspaceTreeNode>;
  projects: ProjectInfo[];
}

export const buildWorkspaceTree = (projects: ProjectInfo[], rootDirectory: string): WorkspaceTreeNode => {
  const root: WorkspaceTreeNode = { name: "", children: new Map(), projects: [] };
  const normalizedRoot = rootDirectory.replace(/\/+$/, "");

  for (const project of projects) {
    if (!project.path.startsWith(normalizedRoot + "/")) {
      continue;
    }

    const relative = project.path.substring(normalizedRoot.length + 1);
    const parts = relative.split("/");
    let current = root;

    for (let i = 0; i < parts.length - 1; i++) {
      const part = parts[i];
      if (!current.children.has(part)) {
        current.children.set(part, { name: part, children: new Map(), projects: [] });
      }
      current = current.children.get(part)!;
    }

    current.projects.push(project);
  }

  return root;
};

export const buildMultiRootWorkspaceTree = (
  projects: ProjectInfo[],
  workspaces: WorkspaceEntry[],
): WorkspaceTreeNode => {
  const root: WorkspaceTreeNode = { name: "", children: new Map(), projects: [] };

  const sorted = [...workspaces]
    .map((ws) => ({ ...ws, directory: ws.directory.replace(/\/+$/, "") }))
    .sort((a, b) => b.directory.length - a.directory.length);

  const projectsByWs = new Map<string, ProjectInfo[]>();
  for (const ws of sorted) {
    projectsByWs.set(ws.directory, []);
  }

  for (const project of projects) {
    const match = sorted.find((ws) => project.path.startsWith(ws.directory + "/"));
    if (match) {
      projectsByWs.get(match.directory)!.push(project);
    }
  }

  for (const ws of sorted) {
    const normalizedDir = ws.directory;
    const dirName = normalizedDir.split("/").pop() || normalizedDir;
    const wsProjects = projectsByWs.get(normalizedDir)!;

    if (ws.scanDepth > 1) {
      const subTree = buildWorkspaceTree(wsProjects, normalizedDir);
      const wsNode: WorkspaceTreeNode = {
        name: dirName,
        children: subTree.children,
        projects: subTree.projects,
      };
      root.children.set(normalizedDir, wsNode);
    } else {
      const wsNode: WorkspaceTreeNode = {
        name: dirName,
        children: new Map(),
        projects: wsProjects,
      };
      root.children.set(normalizedDir, wsNode);
    }
  }

  return root;
};
