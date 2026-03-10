import { useCallback } from "react";
import type { TagEntry } from "../../lib/git-types";

interface TagItemProps {
  tag: TagEntry;
  onDelete: (name: string) => void;
  onPush: (name: string) => void;
  onDeleteRemote?: (name: string) => void;
}

export const TagItem = ({ tag, onDelete, onPush, onDeleteRemote }: TagItemProps) => {
  const handleDelete = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onDelete(tag.name);
  }, [tag.name, onDelete]);

  const handlePush = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onPush(tag.name);
  }, [tag.name, onPush]);

  const handleDeleteRemote = useCallback((e: React.MouseEvent) => {
    e.stopPropagation();
    onDeleteRemote?.(tag.name);
  }, [tag.name, onDeleteRemote]);

  return (
    <div className="branch-item tag-item">
      <span
        className={`codicon ${tag.isAnnotated ? "codicon-bookmark" : "codicon-tag"}`}
        style={{ marginRight: 6, fontSize: 14 }}
      />
      <span className="branch-item-name">{tag.name}</span>
      {tag.message && (
        <span className="tag-item-message" title={tag.message}>
          {tag.message}
        </span>
      )}
      <div className="tag-item-actions">
        <button className="inline-action" onClick={handlePush} title="Push Tag">
          <span className="codicon codicon-cloud-upload" />
        </button>
        {onDeleteRemote && (
          <button className="inline-action" onClick={handleDeleteRemote} title="Delete Remote Tag">
            <span className="codicon codicon-cloud" />
          </button>
        )}
        <button className="inline-action" onClick={handleDelete} title="Delete Tag">
          <span className="codicon codicon-trash" />
        </button>
      </div>
    </div>
  );
};
