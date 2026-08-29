import { createSignal } from "solid-js";
import type { BranchEntry } from "../../lib/git-types";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";

type BranchItemProps = {
  branch: BranchEntry;
  onSelect: () => void;
  onRename?: (name: string) => void;
  onDelete?: (name: string, force: boolean) => void;
  onDeleteRemote?: (remote: string, name: string) => void;
  onMerge?: (name: string) => void;
  onRebase?: (name: string) => void;
};

export const BranchItem = (props: BranchItemProps) => {
  const [contextMenu, setContextMenu] = createSignal<{ x: number; y: number } | null>(null);

  const handleContextMenu = (e: MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  };

  const handleCopyName = () => {
    void navigator.clipboard.writeText(props.branch.name);
  };

  const getContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [
      { label: "Checkout", icon: "check", action: props.onSelect },
      { label: "", action: () => {}, separator: true },
      { label: "Copy Branch Name", icon: "copy", action: handleCopyName },
    ];

    if (!props.branch.isHead && !props.branch.isRemote && props.onMerge) {
      const onMerge = props.onMerge;
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: `Merge into Current Branch`, icon: "git-merge", action: () => onMerge(props.branch.name) }
      );
    }

    if (!props.branch.isHead && !props.branch.isRemote && props.onRebase) {
      const onRebase = props.onRebase;
      items.push({
        label: `Rebase onto ${props.branch.name}`,
        icon: "git-pull-request",
        action: () => onRebase(props.branch.name),
      });
    }

    if (!props.branch.isRemote) {
      if (props.onRename) {
        const onRename = props.onRename;
        items.push(
          { label: "", action: () => {}, separator: true },
          { label: "Rename Branch...", icon: "edit", action: () => onRename(props.branch.name) }
        );
      }

      if (!props.branch.isHead && props.onDelete) {
        const onDelete = props.onDelete;
        items.push({ label: "Delete Branch", icon: "trash", action: () => onDelete(props.branch.name, false) });
      }
    }

    if (props.branch.isRemote && props.onDeleteRemote) {
      const onDeleteRemote = props.onDeleteRemote;
      const parts = props.branch.name.split("/");
      const remote = parts[0];
      const branchName = parts.slice(1).join("/");
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Delete Remote Branch", icon: "trash", action: () => onDeleteRemote(remote, branchName) }
      );
    }

    return items;
  };

  const iconClass = () =>
    props.branch.isHead ? "codicon-check" : props.branch.isRemote ? "codicon-cloud" : "codicon-git-branch";

  return (
    <>
      <div class="branch-item" onClick={props.onSelect} onContextMenu={handleContextMenu}>
        <span class={`codicon ${iconClass()}`} style={{ "margin-right": "6px", "font-size": "14px" }} />
        <span class="branch-item-name">{props.branch.name}</span>
        {props.branch.ahead > 0 && (
          <span class="branch-item-badge">
            {props.branch.ahead}
            {"\u2191"}
          </span>
        )}
        {props.branch.behind > 0 && (
          <span class="branch-item-badge">
            {props.branch.behind}
            {"\u2193"}
          </span>
        )}
      </div>
      {contextMenu() && (
        <ContextMenu
          x={contextMenu()!.x}
          y={contextMenu()!.y}
          items={getContextMenuItems()}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
};
