import { createMemo, createSignal, For } from "solid-js";
import type { FileEntry, ResourceGroupKind } from "../../lib/git-types";
import { ResourceItem } from "./resource-item";
import { getFileIconClasses } from "../../lib/icon-themes/get-icon-classes";

type TreeNode = {
  name: string;
  fullPath: string;
  children: Map<string, TreeNode>;
  files: FileEntry[];
};

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

type TreeFolderProps = {
  node: TreeNode;
  groupKind: ResourceGroupKind;
  depth: number;
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

  return (
    <div>
      {props.node.name && (
        <div
          class="resource-tree-folder"
          style={{ "padding-left": `${12 + props.depth * 12}px` }}
          onClick={() => setCollapsed(!collapsed())}
        >
          <span class={`codicon codicon-chevron-down resource-group-chevron ${collapsed() ? "collapsed" : ""}`} />
          <span class={`resource-item-icon ${getFileIconClasses(props.node.name, "folder")}`} />
          <span class="resource-tree-folder-name">{props.node.name}</span>
        </div>
      )}
      {!collapsed() && (
        <>
          <For each={sortedChildren()} keyed={(child) => child.fullPath}>
            {(child) => <TreeFolder node={child()} groupKind={props.groupKind} depth={childDepth()} />}
          </For>
          <For each={sortedFiles()} keyed={(file) => file.path}>
            {(file) => <ResourceItem file={file()} groupKind={props.groupKind} treeDepth={childDepth()} />}
          </For>
        </>
      )}
    </div>
  );
};

type ResourceTreeProps = {
  files: FileEntry[];
  groupKind: ResourceGroupKind;
};

export const ResourceTree = (props: ResourceTreeProps) => {
  const tree = createMemo(() => buildTree(props.files));
  return <TreeFolder node={tree()} groupKind={props.groupKind} depth={0} />;
};
