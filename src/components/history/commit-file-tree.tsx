import { createMemo, createSignal, For } from "solid-js";
import type { CommitFileEntry } from "../../lib/git-types";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";

type TreeNode = {
  name: string;
  fullPath: string;
  children: Map<string, TreeNode>;
  files: CommitFileEntry[];
};

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

type TreeFolderProps = {
  node: TreeNode;
  depth: number;
  selectedPath: string | null;
  onFileClick: (path: string) => void;
};

const TreeFolder = (props: TreeFolderProps) => {
  const [collapsed, setCollapsed] = createSignal(false);

  const sortedChildren = createMemo(() =>
    Array.from(props.node.children.values()).sort((a, b) => a.name.localeCompare(b.name))
  );
  const sortedFiles = createMemo(() =>
    [...props.node.files].sort((a, b) => {
      const nameA = a.path.split("/").pop() ?? a.path;
      const nameB = b.path.split("/").pop() ?? b.path;
      return nameA.localeCompare(nameB);
    })
  );

  const childDepth = () => (props.node.name ? props.depth + 1 : props.depth);
  const folderPad = () => 12 + props.depth * 12;
  const filePad = () => 12 + childDepth() * 12;

  return (
    <div>
      {props.node.name && (
        <div
          class="commit-tree-folder"
          style={{ "padding-left": `${folderPad()}px` }}
          onClick={() => setCollapsed((value) => !value)}
        >
          <span class={["codicon", "codicon-chevron-down", "resource-group-chevron", { collapsed: collapsed() }]} />
          <span class={["commit-detail-file-icon", getFileIconClasses(props.node.name, "folder")]} />
          <span class="commit-tree-folder-name">{props.node.name}</span>
        </div>
      )}
      {!collapsed() && (
        <>
          <For each={sortedChildren()} keyed={(child) => child.fullPath}>
            {(child) => (
              <TreeFolder
                node={child()}
                depth={childDepth()}
                selectedPath={props.selectedPath}
                onFileClick={props.onFileClick}
              />
            )}
          </For>
          <For each={sortedFiles()} keyed={(file) => file.path}>
            {(file) => {
              const fileName = () => file().path.split("/").pop() ?? file().path;
              const oldName = () => {
                const oldPath = file().oldPath;
                return oldPath ? (oldPath.split("/").pop() ?? oldPath) : null;
              };
              return (
                <div
                  class={["commit-detail-file", { selected: props.selectedPath === file().path }]}
                  style={{ "padding-left": `${filePad()}px` }}
                  onClick={() => props.onFileClick(file().path)}
                >
                  <span class={["commit-detail-file-icon", getFileIconClasses(file().path, "file")]} />
                  <span class="commit-detail-file-path" title={file().path}>
                    {oldName() ? `${oldName()} -> ${fileName()}` : fileName()}
                  </span>
                  <span class={["commit-file-badge", `badge-${file().status}`]}>
                    {STATUS_LETTER[file().status] ?? "M"}
                  </span>
                </div>
              );
            }}
          </For>
        </>
      )}
    </div>
  );
};

type CommitFileTreeProps = {
  files: CommitFileEntry[];
  selectedPath: string | null;
  onFileClick: (path: string) => void;
};

export const CommitFileTree = (props: CommitFileTreeProps) => {
  const tree = createMemo(() => buildTree(props.files));
  return <TreeFolder node={tree()} depth={0} selectedPath={props.selectedPath} onFileClick={props.onFileClick} />;
};
