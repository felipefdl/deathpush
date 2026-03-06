import { useState, useEffect, useRef, useCallback } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useBranches } from "../../hooks/use-branches";
import { useTags } from "../../hooks/use-tags";
import { BranchItem } from "./branch-item";
import { TagItem } from "./tag-item";

interface BranchPickerProps {
  onClose: () => void;
}

export const BranchPicker = ({ onClose }: BranchPickerProps) => {
  const [search, setSearch] = useState("");
  const [tagsExpanded, setTagsExpanded] = useState(false);
  const { branches, tags } = useRepositoryStore();
  const { loadBranches, switchBranch, createNewBranch } = useBranches();
  const { loadTags, createTag, removeTag, pushTagToRemote } = useTags();
  const inputRef = useRef<HTMLInputElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadBranches();
    loadTags();
    inputRef.current?.focus();
  }, [loadBranches, loadTags]);

  const filtered = branches.filter((b) =>
    b.name.toLowerCase().includes(search.toLowerCase()),
  );

  const filteredTags = tags.filter((t) =>
    t.name.toLowerCase().includes(search.toLowerCase()),
  );

  const handleSelect = useCallback(async (name: string) => {
    await switchBranch(name);
    onClose();
  }, [switchBranch, onClose]);

  const handleCreate = useCallback(async () => {
    if (!search.trim()) return;
    await createNewBranch(search.trim());
    onClose();
  }, [search, createNewBranch, onClose]);

  const handleCreateTag = useCallback(async () => {
    if (!search.trim()) return;
    await createTag(search.trim());
    setSearch("");
  }, [search, createTag]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "Escape") {
      onClose();
    } else if (e.key === "Enter" && filtered.length > 0) {
      handleSelect(filtered[0].name);
    }
  }, [onClose, filtered, handleSelect]);

  const handleOverlayClick = useCallback((e: React.MouseEvent) => {
    if (e.target === overlayRef.current) {
      onClose();
    }
  }, [onClose]);

  return (
    <div className="branch-picker-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="branch-picker">
        <input
          ref={inputRef}
          className="branch-picker-input"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Switch to branch..."
        />
        <div className="branch-picker-list">
          {filtered.map((branch) => (
            <BranchItem
              key={branch.name}
              branch={branch}
              onSelect={() => handleSelect(branch.name)}
            />
          ))}
          {search.trim() && !filtered.some((b) => b.name === search.trim()) && (
            <div className="branch-picker-create" onClick={handleCreate}>
              <span className="codicon codicon-add" />
              <span>Create branch: {search.trim()}</span>
            </div>
          )}
          <div
            className="branch-picker-section-header"
            onClick={() => setTagsExpanded(!tagsExpanded)}
          >
            <span className={`codicon codicon-chevron-${tagsExpanded ? "down" : "right"}`} />
            <span>Tags ({filteredTags.length})</span>
          </div>
          {tagsExpanded && (
            <>
              {filteredTags.map((tag) => (
                <TagItem
                  key={tag.name}
                  tag={tag}
                  onDelete={removeTag}
                  onPush={pushTagToRemote}
                />
              ))}
              {search.trim() && !filteredTags.some((t) => t.name === search.trim()) && (
                <div className="branch-picker-create" onClick={handleCreateTag}>
                  <span className="codicon codicon-add" />
                  <span>Create tag: {search.trim()}</span>
                </div>
              )}
              {filteredTags.length === 0 && !search.trim() && (
                <div className="branch-picker-empty">No tags</div>
              )}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
