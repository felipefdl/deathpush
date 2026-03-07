import { useMemo, useState } from "react";
import type { CommitFileEntry } from "../../lib/git-types";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";

interface TreeNode {
  name: string;
  fullPath: string;
  children: Map<string, TreeNode>;
  files: CommitFileEntry[];
}

const buildTree = (files: CommitFileEntry[]): TreeNode => {
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

const STATUS_LETTER: Record<string, string> = {
  added: "A",
  deleted: "D",
  modified: "M",
  renamed: "R",
  copied: "C",
  typeChanged: "T",
};

interface TreeFolderProps {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onFileClick: (path: string) => void;
}

const TreeFolder = ({ node, depth, selectedPath, onFileClick }: TreeFolderProps) => {
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
          className="commit-tree-folder"
          style={{ paddingLeft: 12 + depth * 12 }}
          onClick={() => setCollapsed(!collapsed)}
        >
          <span className={`codicon codicon-chevron-down resource-group-chevron ${collapsed ? "collapsed" : ""}`} />
          <span className={`commit-detail-file-icon ${getFileIconClasses(node.name, "folder")}`} />
          <span className="commit-tree-folder-name">{node.name}</span>
        </div>
      )}
      {!collapsed && (
        <>
          {sortedChildren.map((child) => (
            <TreeFolder
              key={child.fullPath}
              node={child}
              depth={node.name ? depth + 1 : depth}
              selectedPath={selectedPath}
              onFileClick={onFileClick}
            />
          ))}
          {sortedFiles.map((file) => {
            const fileName = file.path.split("/").pop() ?? file.path;
            const isSelected = selectedPath === file.path;
            return (
              <div
                key={file.path}
                className={`commit-detail-file${isSelected ? " selected" : ""}`}
                style={{ paddingLeft: 12 + (node.name ? depth + 1 : depth) * 12 }}
                onClick={() => onFileClick(file.path)}
              >
                <span className={`commit-detail-file-icon ${getFileIconClasses(file.path, "file")}`} />
                <span className="commit-detail-file-path" title={file.path}>
                  {file.oldPath ? `${file.oldPath.split("/").pop()} -> ${fileName}` : fileName}
                </span>
                <span className={`commit-file-badge badge-${file.status}`}>
                  {STATUS_LETTER[file.status] ?? "M"}
                </span>
              </div>
            );
          })}
        </>
      )}
    </div>
  );
};

interface CommitFileTreeProps {
  files: CommitFileEntry[];
  selectedPath: string | null;
  onFileClick: (path: string) => void;
}

export const CommitFileTree = ({ files, selectedPath, onFileClick }: CommitFileTreeProps) => {
  const tree = useMemo(() => buildTree(files), [files]);
  return <TreeFolder node={tree} depth={0} selectedPath={selectedPath} onFileClick={onFileClick} />;
};
