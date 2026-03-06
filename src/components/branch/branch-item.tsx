import type { BranchEntry } from "../../lib/git-types";

interface BranchItemProps {
  branch: BranchEntry;
  onSelect: () => void;
}

export const BranchItem = ({ branch, onSelect }: BranchItemProps) => {
  return (
    <div className="branch-item" onClick={onSelect}>
      <span
        className={`codicon ${branch.isHead ? "codicon-check" : branch.isRemote ? "codicon-cloud" : "codicon-git-branch"}`}
        style={{ marginRight: 6, fontSize: 14 }}
      />
      <span className="branch-item-name">{branch.name}</span>
      {branch.ahead > 0 && <span className="branch-item-badge">{branch.ahead}\u2191</span>}
      {branch.behind > 0 && <span className="branch-item-badge">{branch.behind}\u2193</span>}
    </div>
  );
};
