import type { StashEntry } from "../../lib/git-types";

type StashEntryRowProps = {
  entry: StashEntry;
  onApply: (index: number) => void;
  onPop: (index: number) => void;
  onDrop: (index: number) => void;
  onShow?: (index: number) => void;
};

export const StashEntryRow = (props: StashEntryRowProps) => {
  return (
    <div class="resource-item">
      <span class="resource-item-icon">
        <span class="codicon codicon-archive" />
      </span>
      <span class="resource-item-name" title={props.entry.message}>
        {props.entry.message}
      </span>
      <div class="resource-item-actions">
        {props.onShow && (
          <button class="inline-action" onClick={() => props.onShow?.(props.entry.index)} title="Show Stash Contents">
            <span class="codicon codicon-eye" />
          </button>
        )}
        <button class="inline-action" onClick={() => props.onApply(props.entry.index)} title="Apply Stash">
          <span class="codicon codicon-check" />
        </button>
        <button class="inline-action" onClick={() => props.onPop(props.entry.index)} title="Pop Stash">
          <span class="codicon codicon-arrow-up" />
        </button>
        <button class="inline-action" onClick={() => props.onDrop(props.entry.index)} title="Drop Stash">
          <span class="codicon codicon-trash" />
        </button>
      </div>
    </div>
  );
};
