import { useState, useEffect, useCallback } from "react";
import type { StashEntry, FileDiffWithHunks } from "../../lib/git-types";
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

export const StashBody = ({ stashes, onApply, onPop, onDrop }: StashBodyProps) => {
  const { showStash } = useStash();
  const [expandedStash, setExpandedStash] = useState<number | null>(null);
  const [stashDiff, setStashDiff] = useState<FileDiffWithHunks | null>(null);

  const handleShow = useCallback(async (index: number) => {
    if (expandedStash === index) {
      setExpandedStash(null);
      setStashDiff(null);
      return;
    }
    const result = await showStash(index);
    if (result) {
      setStashDiff(result);
      setExpandedStash(index);
    }
  }, [expandedStash, showStash]);

  return (
    <div className="resource-group-body">
      {stashes.map((entry) => (
        <div key={entry.index}>
          <StashEntryRow
            entry={entry}
            onApply={onApply}
            onPop={onPop}
            onDrop={onDrop}
            onShow={handleShow}
          />
          {expandedStash === entry.index && stashDiff && (
            <div className="stash-diff-preview">
              {stashDiff.hunks.map((hunk, i) => (
                <div key={i} className="stash-diff-hunk">
                  <div className="stash-diff-header">{hunk.header}</div>
                  {hunk.lines.map((line, j) => (
                    <div
                      key={j}
                      className={`stash-diff-line stash-diff-line-${line.lineType}`}
                    >
                      {line.lineType === "add" ? "+" : line.lineType === "remove" ? "-" : " "}
                      {line.content}
                    </div>
                  ))}
                </div>
              ))}
              {stashDiff.hunks.length === 0 && (
                <div className="stash-diff-empty">No diff available</div>
              )}
            </div>
          )}
        </div>
      ))}
    </div>
  );
};

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
