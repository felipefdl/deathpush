import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import { confirm } from "@tauri-apps/plugin-dialog";
import { Portal } from "@solidjs/web";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { useStash } from "../../hooks/use-stash";
import { useBranches } from "../../hooks/use-branches";
import { flushAll } from "../../lib/pierre/flush-registry";
import * as commands from "../../lib/tauri-commands";

type OverflowMenuProps = {
  anchorRef: HTMLButtonElement | undefined;
  onClose: () => void;
  onOpenRepository: () => void;
  onCloneRepository?: () => void;
};

export const OverflowMenu = (props: OverflowMenuProps) => {
  let menuRef: HTMLDivElement | undefined;
  const status = useStore(repositoryStore, (s) => s.status);
  const stashes = useStore(repositoryStore, (s) => s.stashes);
  const branches = useStore(repositoryStore, (s) => s.branches);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const { setError, startOperation, endOperation } = repositoryStore.getState();
  const { saveStash, saveStashIncludeUntracked, saveStashStaged, popStash } = useStash();
  const { loadBranches, mergeBranch, rebaseBranch } = useBranches();
  const [showMergePicker, setShowMergePicker] = createSignal(false);
  const [showRebasePicker, setShowRebasePicker] = createSignal(false);
  const [pickerSearch, setPickerSearch] = createSignal("");
  let pickerInputRef: HTMLInputElement | undefined;

  const branch = () => status()?.headBranch;
  const hasStaged = () => status()?.groups.some((g) => g.kind === "index" && g.files.length > 0) ?? false;
  const hasUnstaged = () => status()?.groups.some((g) => g.kind !== "index" && g.files.length > 0) ?? false;
  const hasCommit = () => !!status()?.headCommit;
  const hasStashes = () => stashes().length > 0;
  const noBranch = () => !branch();
  const isNetworkBusy = () => operations().has("push") || operations().has("pull") || operations().has("fetch");
  const showingPicker = () => showMergePicker() || showRebasePicker();

  const menuStyle = () => {
    const margin = 8;
    const preferredWidth = showingPicker() ? 260 : 200;
    const width = Math.min(preferredWidth, Math.max(0, window.innerWidth - margin * 2));
    const anchor = props.anchorRef?.getBoundingClientRect();
    const right = anchor?.right ?? window.innerWidth - margin;
    const maximumLeft = Math.max(margin, window.innerWidth - width - margin);
    const left = Math.min(Math.max(margin, right - width), maximumLeft);
    return {
      position: "fixed" as const,
      left: `${left}px`,
      right: "auto",
      top: `${anchor?.bottom ?? margin}px`,
      "min-width": `${width}px`,
      "max-width": `${width}px`,
    };
  };

  const filteredBranches = createMemo(() => {
    const q = pickerSearch().toLowerCase();
    return branches().filter((b) => !b.isHead && b.name.toLowerCase().includes(q));
  });

  onSettled(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (
        menuRef &&
        !menuRef.contains(e.target as Node) &&
        props.anchorRef &&
        !props.anchorRef.contains(e.target as Node)
      ) {
        props.onClose();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        if (showMergePicker() || showRebasePicker()) {
          setShowMergePicker(false);
          setShowRebasePicker(false);
        } else {
          props.onClose();
        }
      }
    };
    document.addEventListener("mousedown", handleClickOutside);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClickOutside);
      document.removeEventListener("keydown", handleEscape);
    };
  });

  createEffect(
    () => showingPicker(),
    (open) => {
      if (open) {
        void loadBranches();
        pickerInputRef?.focus();
      }
    }
  );

  const handleItem = (action: () => void, disabled?: boolean) => {
    if (disabled) return;
    props.onClose();
    action();
  };

  const handlePull = async (rebase: boolean = false) => {
    const current = branch();
    if (!current) return;
    startOperation("pull");
    try {
      await commands.pull("origin", current, rebase);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("pull");
    }
  };

  const handlePush = async (force: boolean = false) => {
    const current = branch();
    if (!current) return;
    if (force) {
      const confirmed = await confirm("Are you sure you want to force push? This may overwrite remote changes.", {
        title: "Force Push",
        kind: "warning",
        okLabel: "Force Push",
        cancelLabel: "Cancel",
      });
      if (!confirmed) return;
    }
    startOperation("push");
    try {
      await commands.push("origin", current, force);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("push");
    }
  };

  const handleFetch = async () => {
    startOperation("fetch");
    try {
      await commands.fetchRemote("origin", true);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("fetch");
    }
  };

  const handleSync = async () => {
    const current = branch();
    if (!current) return;
    startOperation("pull");
    try {
      await commands.pull("origin", current);
      await commands.push("origin", current);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("pull");
    }
  };

  const handleStageAll = async () => {
    startOperation("stage");
    try {
      await flushAll();
      await commands.stageAll();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("stage");
    }
  };

  const handleUnstageAll = async () => {
    startOperation("unstage");
    try {
      await commands.unstageAll();
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("unstage");
    }
  };

  const handleDiscardAll = async () => {
    const current = repositoryStore.getState().status;
    if (!current) return;
    const unstaged = current.groups.filter((g) => g.kind !== "index");
    const paths = unstaged.flatMap((g) => g.files.map((f) => f.path));
    const count = paths.length;
    if (count === 0) return;
    const confirmed = await confirm(
      `Are you sure you want to discard all ${count} change(s)?\n\nThis action is irreversible.`,
      { title: "Discard All Changes", kind: "warning", okLabel: "Discard All", cancelLabel: "Cancel" }
    );
    if (!confirmed) return;
    startOperation("discard");
    try {
      await flushAll();
      await commands.discardChanges(paths);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("discard");
    }
  };

  const handleUndoLastCommit = async () => {
    const confirmed = await confirm("Undo last commit? Changes will be moved back to staging.", {
      title: "Undo Last Commit",
      kind: "warning",
    });
    if (!confirmed) return;
    try {
      await commands.undoLastCommit();
    } catch (err) {
      setError(String(err));
    }
  };

  const handleMergeSelect = async (name: string) => {
    setShowMergePicker(false);
    props.onClose();
    await mergeBranch(name);
  };

  const handleRebaseSelect = async (name: string) => {
    setShowRebasePicker(false);
    props.onClose();
    await rebaseBranch(name);
  };

  return (
    <Portal>
      {showingPicker() ? (
        <div
          class="overflow-menu overflow-menu-wide"
          ref={(el) => {
            menuRef = el;
          }}
          style={menuStyle()}
        >
          <div class="overflow-menu-picker-header">{showMergePicker() ? "Merge" : "Rebase onto"}</div>
          <input
            ref={(el) => {
              pickerInputRef = el;
            }}
            class="overflow-menu-picker-input"
            type="search"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck={false}
            data-form-type="other"
            value={pickerSearch()}
            onInput={(e) => setPickerSearch(e.currentTarget.value)}
            onKeyDown={(e) => {
              if (e.key === "Escape") {
                setShowMergePicker(false);
                setShowRebasePicker(false);
              } else if (e.key === "Enter" && filteredBranches().length > 0) {
                if (showMergePicker()) void handleMergeSelect(filteredBranches()[0].name);
                else void handleRebaseSelect(filteredBranches()[0].name);
              }
            }}
            placeholder="Select a branch..."
          />
          <div class="overflow-menu-picker-list">
            <For each={filteredBranches()} keyed={(b) => b.name}>
              {(b) => (
                <div
                  class="context-menu-item"
                  onClick={() => void (showMergePicker() ? handleMergeSelect(b().name) : handleRebaseSelect(b().name))}
                >
                  <span
                    class={`codicon ${b().isRemote ? "codicon-cloud" : "codicon-git-branch"}`}
                    style={{ "margin-right": "8px", "font-size": "14px" }}
                  />
                  {b().name}
                </div>
              )}
            </For>
            {filteredBranches().length === 0 && <div class="context-menu-item disabled">No matching branches</div>}
          </div>
        </div>
      ) : (
        <div
          class="overflow-menu"
          ref={(el) => {
            menuRef = el;
          }}
          style={menuStyle()}
        >
          <div
            class={`context-menu-item${noBranch() || isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(() => handlePull(), noBranch() || isNetworkBusy())}
          >
            Pull
          </div>
          <div
            class={`context-menu-item${noBranch() || isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(() => handlePull(true), noBranch() || isNetworkBusy())}
          >
            Pull (Rebase)
          </div>
          <div
            class={`context-menu-item${noBranch() || isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(() => handlePush(), noBranch() || isNetworkBusy())}
          >
            Push
          </div>
          <div
            class={`context-menu-item${noBranch() || isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(() => handlePush(true), noBranch() || isNetworkBusy())}
          >
            Push (Force)
          </div>
          <div
            class={`context-menu-item${isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(handleFetch, isNetworkBusy())}
          >
            Fetch
          </div>
          <div
            class={`context-menu-item${noBranch() || isNetworkBusy() ? " disabled" : ""}`}
            onClick={() => handleItem(handleSync, noBranch() || isNetworkBusy())}
          >
            Sync
          </div>

          <div class="context-menu-separator" />

          <div
            class={`context-menu-item${noBranch() ? " disabled" : ""}`}
            onClick={() => {
              if (!noBranch()) {
                setShowMergePicker(true);
                setPickerSearch("");
              }
            }}
          >
            Merge Branch...
          </div>
          <div
            class={`context-menu-item${noBranch() ? " disabled" : ""}`}
            onClick={() => {
              if (!noBranch()) {
                setShowRebasePicker(true);
                setPickerSearch("");
              }
            }}
          >
            Rebase Branch...
          </div>

          <div class="context-menu-separator" />

          <div class="context-menu-item" onClick={() => handleItem(handleStageAll)}>
            Stage All Changes
          </div>
          <div
            class={`context-menu-item${!hasStaged() ? " disabled" : ""}`}
            onClick={() => handleItem(handleUnstageAll, !hasStaged())}
          >
            Unstage All Changes
          </div>
          <div
            class={`context-menu-item${!hasUnstaged() ? " disabled" : ""}`}
            onClick={() => handleItem(handleDiscardAll, !hasUnstaged())}
          >
            Discard All Changes
          </div>

          <div class="context-menu-separator" />

          <div class="context-menu-item" onClick={() => handleItem(() => saveStash())}>
            Stash Changes
          </div>
          <div class="context-menu-item" onClick={() => handleItem(() => saveStashIncludeUntracked())}>
            Stash (Include Untracked)
          </div>
          <div
            class={`context-menu-item${!hasStaged() ? " disabled" : ""}`}
            onClick={() => handleItem(() => saveStashStaged(), !hasStaged())}
          >
            Stash Staged Only
          </div>
          <div
            class={`context-menu-item${!hasStashes() ? " disabled" : ""}`}
            onClick={() => handleItem(() => popStash(0), !hasStashes())}
          >
            Stash Pop (Latest)
          </div>

          <div class="context-menu-separator" />

          <div
            class={`context-menu-item${!hasCommit() ? " disabled" : ""}`}
            onClick={() => handleItem(handleUndoLastCommit, !hasCommit())}
          >
            Undo Last Commit
          </div>

          <div class="context-menu-separator" />

          <div class="context-menu-item" onClick={() => handleItem(props.onOpenRepository)}>
            Open Repository...
          </div>
          {props.onCloneRepository && (
            <div class="context-menu-item" onClick={() => handleItem(props.onCloneRepository!)}>
              Clone Repository...
            </div>
          )}
        </div>
      )}
    </Portal>
  );
};
