import { useState } from "react";
import type { FileEntry, ResourceGroupKind } from "../../lib/git-types";
import { ResourceItem } from "./resource-item";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";

interface TreeNode {
  name: string;
  fullPath: string;
  children: Map<string, TreeNode>;
  files: FileEntry[];
}

const buildTree = (files: FileEntry[]): TreeNode => {
  const root: TreeNode = { name: "", fullPath: "", children: new Map(), files: [] };

  for (const file of files) {
    const parts = file.path.split("/");
    let current = root;

    for (let i = 0; i < parts.length - 1; i++) {
      const part = parts[i];
      if (!current.children.has(part)) {
        const fullPath = parts.slice(0, i + 1).join("/");
        current.children.set(part, { name: part, fullPath, children: new Map(), files: [] });
      }
      current = current.children.get(part)!;
    }

    current.files.push(file);
  }

  return root;
};

interface TreeFolderProps {
  node: TreeNode;
  groupKind: ResourceGroupKind;
  depth: number;
}

const TreeFolder = ({ node, groupKind, depth }: TreeFolderProps) => {
  const [collapsed, setCollapsed] = useState(false);

  const sortedChildren = Array.from(node.children.values()).sort((a, b) => a.name.localeCompare(b.name));
  const sortedFiles = [...node.files].sort((a, b) => {
    const nameA = a.path.split("/").pop() ?? a.path;
    const nameB = b.path.split("/").pop() ?? b.path;
    return nameA.localeCompare(nameB);
  });

  return (
    <div>
      {node.name && (
        <div
          className="resource-tree-folder"
          style={{ paddingLeft: 12 + depth * 12 }}
          onClick={() => setCollapsed(!collapsed)}
        >
          <span className={`codicon codicon-chevron-down resource-group-chevron ${collapsed ? "collapsed" : ""}`} />
          <span className={`resource-item-icon ${getFileIconClasses(node.name, "folder")}`} />
          <span className="resource-tree-folder-name">{node.name}</span>
        </div>
      )}
      {!collapsed && (
        <>
          {sortedChildren.map((child) => (
            <TreeFolder key={child.fullPath} node={child} groupKind={groupKind} depth={node.name ? depth + 1 : depth} />
          ))}
          {sortedFiles.map((file) => (
            <ResourceItem key={file.path} file={file} groupKind={groupKind} treeDepth={node.name ? depth + 1 : depth} />
          ))}
        </>
      )}
    </div>
  );
};

interface ResourceTreeProps {
  files: FileEntry[];
  groupKind: ResourceGroupKind;
}

export const ResourceTree = ({ files, groupKind }: ResourceTreeProps) => {
  const tree = buildTree(files);
  return <TreeFolder node={tree} groupKind={groupKind} depth={0} />;
};
