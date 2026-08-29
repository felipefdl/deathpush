import type { TagEntry } from "../../lib/git-types";

type TagItemProps = {
  tag: TagEntry;
  onDelete: (name: string) => void;
  onPush: (name: string) => void;
  onDeleteRemote?: (name: string) => void;
};

export const TagItem = (props: TagItemProps) => {
  const handleDelete = (e: MouseEvent) => {
    e.stopPropagation();
    props.onDelete(props.tag.name);
  };

  const handlePush = (e: MouseEvent) => {
    e.stopPropagation();
    props.onPush(props.tag.name);
  };

  const handleDeleteRemote = (e: MouseEvent) => {
    e.stopPropagation();
    props.onDeleteRemote?.(props.tag.name);
  };

  return (
    <div class="branch-item tag-item">
      <span
        class={`codicon ${props.tag.isAnnotated ? "codicon-bookmark" : "codicon-tag"}`}
        style={{ "margin-right": "6px", "font-size": "14px" }}
      />
      <span class="branch-item-name">{props.tag.name}</span>
      {props.tag.message && (
        <span class="tag-item-message" title={props.tag.message}>
          {props.tag.message}
        </span>
      )}
      <div class="tag-item-actions">
        <button class="inline-action" onClick={handlePush} title="Push Tag">
          <span class="codicon codicon-cloud-upload" />
        </button>
        {props.onDeleteRemote && (
          <button class="inline-action" onClick={handleDeleteRemote} title="Delete Remote Tag">
            <span class="codicon codicon-cloud" />
          </button>
        )}
        <button class="inline-action" onClick={handleDelete} title="Delete Tag">
          <span class="codicon codicon-trash" />
        </button>
      </div>
    </div>
  );
};
