import { useState, useEffect, useRef, useCallback } from "react";
import { confirm } from "@tauri-apps/plugin-dialog";
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
  const [renaming, setRenaming] = useState<string | null>(null);
  const [renameValue, setRenameValue] = useState("");
  const { branches, tags } = useRepositoryStore();
  const {
    loadBranches,
    switchBranch,
    createNewBranch,
    renameBranch,
    removeBranch,
    removeRemoteBranch,
    mergeBranch,
    rebaseBranch,
  } = useBranches();
  const { loadTags, createTag, removeTag, pushTagToRemote, removeRemoteTag } = useTags();
  const inputRef = useRef<HTMLInputElement>(null);
  const renameInputRef = useRef<HTMLInputElement>(null);
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loadBranches();
    loadTags();
    inputRef.current?.focus();
  }, [loadBranches, loadTags]);

  useEffect(() => {
    if (renaming) {
      renameInputRef.current?.focus();
      renameInputRef.current?.select();
    }
  }, [renaming]);

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

  const handleStartRename = useCallback((name: string) => {
    setRenaming(name);
    setRenameValue(name);
  }, []);

  const handleConfirmRename = useCallback(async () => {
    if (!renaming || !renameValue.trim() || renameValue.trim() === renaming) {
      setRenaming(null);
      return;
    }
    await renameBranch(renaming, renameValue.trim());
    setRenaming(null);
  }, [renaming, renameValue, renameBranch]);

  const handleDeleteBranch = useCallback(async (name: string, force: boolean) => {
    const confirmed = await confirm(
      `Are you sure you want to delete branch "${name}"?`,
      { title: "Delete Branch", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    await removeBranch(name, force);
  }, [removeBranch]);

  const handleDeleteRemoteBranch = useCallback(async (remote: string, name: string) => {
    const confirmed = await confirm(
      `Are you sure you want to delete remote branch "${remote}/${name}"?\n\nThis cannot be undone.`,
      { title: "Delete Remote Branch", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    await removeRemoteBranch(remote, name);
  }, [removeRemoteBranch]);

  const handleMerge = useCallback(async (name: string) => {
    await mergeBranch(name);
    onClose();
  }, [mergeBranch, onClose]);

  const handleRebase = useCallback(async (name: string) => {
    await rebaseBranch(name);
    onClose();
  }, [rebaseBranch, onClose]);

  const handleDeleteRemoteTag = useCallback(async (name: string) => {
    const confirmed = await confirm(
      `Are you sure you want to delete remote tag "${name}"?\n\nThis cannot be undone.`,
      { title: "Delete Remote Tag", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" },
    );
    if (!confirmed) return;
    await removeRemoteTag(name);
  }, [removeRemoteTag]);

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
          type="search"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          data-form-type="other"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Switch to branch..."
        />
        <div className="branch-picker-list">
          {filtered.map((branch) => (
            renaming === branch.name ? (
              <div key={branch.name} className="branch-item branch-rename-row">
                <span className="codicon codicon-edit" style={{ marginRight: 6, fontSize: 14 }} />
                <input
                  ref={renameInputRef}
                  className="branch-rename-input"
                  value={renameValue}
                  onChange={(e) => setRenameValue(e.target.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleConfirmRename();
                    if (e.key === "Escape") setRenaming(null);
                  }}
                  onBlur={handleConfirmRename}
                />
              </div>
            ) : (
              <BranchItem
                key={branch.name}
                branch={branch}
                onSelect={() => handleSelect(branch.name)}
                onRename={handleStartRename}
                onDelete={handleDeleteBranch}
                onDeleteRemote={handleDeleteRemoteBranch}
                onMerge={handleMerge}
                onRebase={handleRebase}
              />
            )
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
                  onDeleteRemote={handleDeleteRemoteTag}
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
