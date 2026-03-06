import type { ProjectInfo } from "./tauri-commands";

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
