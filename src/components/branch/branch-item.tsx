import { useCallback, useState } from "react";
import type { BranchEntry } from "../../lib/git-types";
import { ContextMenu, type ContextMenuItem } from "../scm/context-menu";

interface BranchItemProps {
  branch: BranchEntry;
  onSelect: () => void;
  onRename?: (name: string) => void;
  onDelete?: (name: string, force: boolean) => void;
  onDeleteRemote?: (remote: string, name: string) => void;
  onMerge?: (name: string) => void;
  onRebase?: (name: string) => void;
}

export const BranchItem = ({
  branch,
  onSelect,
  onRename,
  onDelete,
  onDeleteRemote,
  onMerge,
  onRebase,
}: BranchItemProps) => {
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number } | null>(null);

  const handleContextMenu = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    e.stopPropagation();
    setContextMenu({ x: e.clientX, y: e.clientY });
  }, []);

  const handleCopyName = useCallback(() => {
    navigator.clipboard.writeText(branch.name);
  }, [branch.name]);

  const getContextMenuItems = (): ContextMenuItem[] => {
    const items: ContextMenuItem[] = [
      { label: "Checkout", icon: "check", action: onSelect },
      { label: "", action: () => {}, separator: true },
      { label: "Copy Branch Name", icon: "copy", action: handleCopyName },
    ];

    if (!branch.isHead && !branch.isRemote && onMerge) {
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: `Merge into Current Branch`, icon: "git-merge", action: () => onMerge(branch.name) },
      );
    }

    if (!branch.isHead && !branch.isRemote && onRebase) {
      items.push(
        { label: `Rebase onto ${branch.name}`, icon: "git-pull-request", action: () => onRebase(branch.name) },
      );
    }

    if (!branch.isRemote) {
      if (onRename) {
        items.push(
          { label: "", action: () => {}, separator: true },
          { label: "Rename Branch...", icon: "edit", action: () => onRename(branch.name) },
        );
      }

      if (!branch.isHead && onDelete) {
        items.push(
          { label: "Delete Branch", icon: "trash", action: () => onDelete(branch.name, false) },
        );
      }
    }

    if (branch.isRemote && onDeleteRemote) {
      const parts = branch.name.split("/");
      const remote = parts[0];
      const branchName = parts.slice(1).join("/");
      items.push(
        { label: "", action: () => {}, separator: true },
        { label: "Delete Remote Branch", icon: "trash", action: () => onDeleteRemote(remote, branchName) },
      );
    }

    return items;
  };

  return (
    <>
      <div className="branch-item" onClick={onSelect} onContextMenu={handleContextMenu}>
        <span
          className={`codicon ${branch.isHead ? "codicon-check" : branch.isRemote ? "codicon-cloud" : "codicon-git-branch"}`}
          style={{ marginRight: 6, fontSize: 14 }}
        />
        <span className="branch-item-name">{branch.name}</span>
        {branch.ahead > 0 && <span className="branch-item-badge">{branch.ahead}{"\u2191"}</span>}
        {branch.behind > 0 && <span className="branch-item-badge">{branch.behind}{"\u2193"}</span>}
      </div>
      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={getContextMenuItems()}
          onClose={() => setContextMenu(null)}
        />
      )}
    </>
  );
};
