import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import { repositoryStore } from "../../stores/repository-store";
import { useBranches } from "../../hooks/use-branches";
import { useTags } from "../../hooks/use-tags";
import { useStore } from "../../lib/use-store";
import { BranchItem } from "./branch-item";
import { TagItem } from "./tag-item";

type BranchPickerProps = {
  onClose: () => void;
};

export const BranchPicker = (props: BranchPickerProps) => {
  const [search, setSearch] = createSignal("");
  const [tagsExpanded, setTagsExpanded] = createSignal(false);
  const [renaming, setRenaming] = createSignal<string | null>(null);
  const [renameValue, setRenameValue] = createSignal("");
  const branches = useStore(repositoryStore, (s) => s.branches);
  const tags = useStore(repositoryStore, (s) => s.tags);
  const { switchBranch, createNewBranch, renameBranch, removeBranch, removeRemoteBranch, mergeBranch, rebaseBranch } =
    useBranches();
  const { createTag, removeTag, pushTagToRemote, removeRemoteTag } = useTags();
  let inputRef: HTMLInputElement | undefined;
  let renameInputRef: HTMLInputElement | undefined;
  let overlayRef: HTMLDivElement | undefined;

  onSettled(() => {
    inputRef?.focus();
  });

  createEffect(
    () => renaming(),
    (name) => {
      if (name) {
        renameInputRef?.focus();
        renameInputRef?.select();
      }
    }
  );

  const filtered = createMemo(() => branches().filter((b) => b.name.toLowerCase().includes(search().toLowerCase())));

  const filteredTags = createMemo(() => tags().filter((t) => t.name.toLowerCase().includes(search().toLowerCase())));

  const handleSelect = async (name: string) => {
    await switchBranch(name);
    props.onClose();
  };

  const handleCreate = async () => {
    const name = search().trim();
    if (!name) return;
    await createNewBranch(name);
    props.onClose();
  };

  const handleCreateTag = async () => {
    const name = search().trim();
    if (!name) return;
    await createTag(name);
    setSearch("");
  };

  const handleStartRename = (name: string) => {
    setRenaming(name);
    setRenameValue(name);
  };

  const handleConfirmRename = async () => {
    const current = renaming();
    const value = renameValue().trim();
    if (!current || !value || value === current) {
      setRenaming(null);
      return;
    }
    await renameBranch(current, value);
    setRenaming(null);
  };

  const handleDeleteBranch = async (name: string, force: boolean) => {
    await removeBranch(name, force);
  };

  const handleDeleteRemoteBranch = async (_remote: string, name: string) => {
    const confirmed = await confirm(
      `Are you sure you want to delete remote branch "${name}"?\n\nThis cannot be undone.`,
      { title: "Delete Remote Branch", kind: "warning", okLabel: "Delete", cancelLabel: "Cancel" }
    );
    if (!confirmed) return;
    await removeRemoteBranch(name);
  };

  const handleMerge = async (name: string) => {
    await mergeBranch(name);
    props.onClose();
  };

  const handleRebase = async (name: string) => {
    await rebaseBranch(name);
    props.onClose();
  };

  const handleDeleteRemoteTag = async (name: string) => {
    const confirmed = await confirm(`Are you sure you want to delete remote tag "${name}"?\n\nThis cannot be undone.`, {
      title: "Delete Remote Tag",
      kind: "warning",
      okLabel: "Delete",
      cancelLabel: "Cancel",
    });
    if (!confirmed) return;
    await removeRemoteTag(name);
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.onClose();
    } else if (e.key === "Enter" && filtered().length > 0) {
      void handleSelect(filtered()[0].name);
    }
  };

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === overlayRef) {
      props.onClose();
    }
  };

  const handleSearchInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    setSearch(e.currentTarget.value);
  };

  const handleRenameInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    setRenameValue(e.currentTarget.value);
  };

  const searchTrimmed = createMemo(() => search().trim());
  const canCreateBranch = createMemo(() => !!searchTrimmed() && !filtered().some((b) => b.name === searchTrimmed()));
  const canCreateTag = createMemo(() => !!searchTrimmed() && !filteredTags().some((t) => t.name === searchTrimmed()));

  return (
    <div
      class="branch-picker-overlay"
      ref={(el) => {
        overlayRef = el;
      }}
      onClick={handleOverlayClick}
    >
      <div class="branch-picker">
        <input
          ref={(el) => {
            inputRef = el;
          }}
          class="branch-picker-input"
          type="search"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck={false}
          data-form-type="other"
          value={search()}
          onInput={handleSearchInput}
          onKeyDown={handleKeyDown}
          placeholder="Switch to branch..."
        />
        <div class="branch-picker-list">
          <For each={filtered()} keyed={(branch) => branch.name}>
            {(branch) =>
              renaming() === branch().name ? (
                <div class="branch-item branch-rename-row">
                  <span class="codicon codicon-edit" style={{ "margin-right": "6px", "font-size": "14px" }} />
                  <input
                    ref={(el) => {
                      renameInputRef = el;
                    }}
                    class="branch-rename-input"
                    value={renameValue()}
                    onInput={handleRenameInput}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") void handleConfirmRename();
                      if (e.key === "Escape") setRenaming(null);
                    }}
                    onBlur={handleConfirmRename}
                  />
                </div>
              ) : (
                <BranchItem
                  branch={branch()}
                  onSelect={() => handleSelect(branch().name)}
                  onRename={handleStartRename}
                  onDelete={handleDeleteBranch}
                  onDeleteRemote={handleDeleteRemoteBranch}
                  onMerge={handleMerge}
                  onRebase={handleRebase}
                />
              )
            }
          </For>
          {canCreateBranch() && (
            <div class="branch-picker-create" onClick={handleCreate}>
              <span class="codicon codicon-add" />
              <span>Create branch: {searchTrimmed()}</span>
            </div>
          )}
          <div class="branch-picker-section-header" onClick={() => setTagsExpanded(!tagsExpanded())}>
            <span class={`codicon codicon-chevron-${tagsExpanded() ? "down" : "right"}`} />
            <span>Tags ({filteredTags().length})</span>
          </div>
          {tagsExpanded() && (
            <>
              <For each={filteredTags()} keyed={(tag) => tag.name}>
                {(tag) => (
                  <TagItem
                    tag={tag()}
                    onDelete={removeTag}
                    onPush={pushTagToRemote}
                    onDeleteRemote={handleDeleteRemoteTag}
                  />
                )}
              </For>
              {canCreateTag() && (
                <div class="branch-picker-create" onClick={handleCreateTag}>
                  <span class="codicon codicon-add" />
                  <span>Create tag: {searchTrimmed()}</span>
                </div>
              )}
              {filteredTags().length === 0 && !searchTrimmed() && <div class="branch-picker-empty">No tags</div>}
            </>
          )}
        </div>
      </div>
    </div>
  );
};
