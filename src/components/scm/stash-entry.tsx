import type { StashEntry } from "../../lib/git-types";

interface StashEntryRowProps {
  entry: StashEntry;
  onApply: (index: number) => void;
  onPop: (index: number) => void;
  onDrop: (index: number) => void;
  onShow?: (index: number) => void;
}

export const StashEntryRow = ({ entry, onApply, onPop, onDrop, onShow }: StashEntryRowProps) => {
  return (
    <div className="resource-item">
      <span className="resource-item-icon">
        <span className="codicon codicon-archive" />
      </span>
      <span className="resource-item-name" title={entry.message}>
        {entry.message}
      </span>
      <div className="resource-item-actions">
        {onShow && (
          <button className="inline-action" onClick={() => onShow(entry.index)} title="Show Stash Contents">
            <span className="codicon codicon-eye" />
          </button>
        )}
        <button className="inline-action" onClick={() => onApply(entry.index)} title="Apply Stash">
          <span className="codicon codicon-check" />
        </button>
        <button className="inline-action" onClick={() => onPop(entry.index)} title="Pop Stash">
          <span className="codicon codicon-arrow-up" />
        </button>
        <button className="inline-action" onClick={() => onDrop(entry.index)} title="Drop Stash">
          <span className="codicon codicon-trash" />
        </button>
      </div>
    </div>
  );
};
