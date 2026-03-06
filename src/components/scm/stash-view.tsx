import { useState, useEffect } from "react";
import type { StashEntry } from "../../lib/git-types";
import { useRepositoryStore } from "../../stores/repository-store";
import { useStash } from "../../hooks/use-stash";
import { StashEntryRow } from "./stash-entry";

interface StashHeaderProps {
  collapsed: boolean;
  onToggle: () => void;
  count: number;
}

export const StashHeader = ({ collapsed, onToggle, count }: StashHeaderProps) => (
  <div className="resource-group-header" onClick={onToggle}>
    <span className={`codicon codicon-chevron-down resource-group-chevron ${collapsed ? "collapsed" : ""}`} />
    <span className="resource-group-label">Stashes</span>
    <span className="resource-group-count">{count}</span>
  </div>
);

interface StashBodyProps {
  stashes: StashEntry[];
  onApply: (index: number) => void;
  onPop: (index: number) => void;
  onDrop: (index: number) => void;
}

export const StashBody = ({ stashes, onApply, onPop, onDrop }: StashBodyProps) => (
  <div className="resource-group-body">
    {stashes.map((entry) => (
      <StashEntryRow
        key={entry.index}
        entry={entry}
        onApply={onApply}
        onPop={onPop}
        onDrop={onDrop}
      />
    ))}
  </div>
);

export const StashView = () => {
  const [collapsed, setCollapsed] = useState(false);
  const { stashes, status } = useRepositoryStore();
  const { loadStashes, applyStash, popStash, dropStash } = useStash();

  useEffect(() => {
    if (status) {
      loadStashes();
    }
  }, [status, loadStashes]);

  if (!status || stashes.length === 0) return null;

  return (
    <div className="resource-group">
      <StashHeader collapsed={collapsed} onToggle={() => setCollapsed(!collapsed)} count={stashes.length} />
      {!collapsed && (
        <StashBody stashes={stashes} onApply={applyStash} onPop={popStash} onDrop={dropStash} />
      )}
    </div>
  );
};
