import { createSignal, For, onSettled } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import type { WorkspaceEntry } from "../../stores/settings-store";

type WorkspaceConfigModalProps = {
  onClose: () => void;
  workspaces: WorkspaceEntry[];
  onSave: (workspaces: WorkspaceEntry[]) => void;
};

const EMPTY_ENTRY: WorkspaceEntry = { directory: "", scanDepth: 1 };

export const WorkspaceConfigModal = (props: WorkspaceConfigModalProps) => {
  const [entries, setEntries] = createSignal<WorkspaceEntry[]>(
    props.workspaces.length > 0 ? props.workspaces.map((w) => ({ ...w })) : [{ ...EMPTY_ENTRY }]
  );
  let overlayRef: HTMLDivElement | undefined;
  let listRef: HTMLDivElement | undefined;

  onSettled(() => {
    const firstInput = listRef?.querySelector<HTMLInputElement>(".clone-dialog-input");
    firstInput?.focus();
  });

  const handleBrowse = async (index: number) => {
    const selected = await open({ directory: true, title: "Select Git Projects Directory" });
    if (selected) {
      setEntries((prev) => prev.map((e, i) => (i === index ? { ...e, directory: selected } : e)));
    }
  };

  const handleDirectoryChange = (index: number, value: string) => {
    setEntries((prev) => prev.map((e, i) => (i === index ? { ...e, directory: value } : e)));
  };

  const handleDepthChange = (index: number, delta: number) => {
    setEntries((prev) =>
      prev.map((e, i) => (i === index ? { ...e, scanDepth: Math.min(5, Math.max(1, e.scanDepth + delta)) } : e))
    );
  };

  const handleRemove = (index: number) => {
    setEntries((prev) => prev.filter((_, i) => i !== index));
  };

  const handleAdd = () => {
    setEntries((prev) => [...prev, { ...EMPTY_ENTRY }]);
    requestAnimationFrame(() => {
      const inputs = listRef?.querySelectorAll<HTMLInputElement>(".clone-dialog-input");
      inputs?.[inputs.length - 1]?.focus();
    });
  };

  const handleSave = () => {
    const filtered = entries().filter((e) => e.directory.trim() !== "");
    props.onSave(filtered);
    props.onClose();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.onClose();
    } else if (e.key === "Enter") {
      handleSave();
    }
  };

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === overlayRef) {
      props.onClose();
    }
  };

  return (
    <div
      class="branch-picker-overlay"
      ref={(el) => {
        overlayRef = el;
      }}
      onClick={handleOverlayClick}
    >
      <div class="clone-dialog" onKeyDown={handleKeyDown}>
        <div class="clone-dialog-title">Workspace Settings</div>
        <div class="workspace-config-description">
          Add directories containing your Git repositories. The scan depth controls how many levels deep to search for
          projects within each directory.
        </div>
        <div
          class="workspace-entries"
          ref={(el) => {
            listRef = el;
          }}
        >
          <For each={entries()} keyed={false}>
            {(entry, index) => (
              <div class="workspace-entry-row">
                <input
                  class="clone-dialog-input"
                  autocomplete="off"
                  autocorrect="off"
                  autocapitalize="off"
                  spellcheck={false}
                  data-form-type="other"
                  value={entry().directory}
                  onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) =>
                    handleDirectoryChange(index, e.currentTarget.value)
                  }
                  placeholder="Select a directory..."
                />
                <button class="clone-dialog-browse" onClick={() => handleBrowse(index)} title="Browse...">
                  <span class="codicon codicon-folder-opened" />
                </button>
                <div class="welcome-depth-control">
                  <button
                    class="welcome-depth-btn"
                    disabled={entry().scanDepth <= 1}
                    onClick={() => handleDepthChange(index, -1)}
                  >
                    <span class="codicon codicon-chevron-left" />
                  </button>
                  <span class="welcome-depth-value">{entry().scanDepth}</span>
                  <button
                    class="welcome-depth-btn"
                    disabled={entry().scanDepth >= 5}
                    onClick={() => handleDepthChange(index, 1)}
                  >
                    <span class="codicon codicon-chevron-right" />
                  </button>
                </div>
                {entries().length > 1 && (
                  <button class="workspace-entry-remove" onClick={() => handleRemove(index)} title="Remove">
                    <span class="codicon codicon-close" />
                  </button>
                )}
              </div>
            )}
          </For>
        </div>
        <button class="workspace-add-btn" onClick={handleAdd}>
          <span class="codicon codicon-add" />
          Add Directory
        </button>
        <div class="clone-dialog-actions">
          <button class="action-button secondary" onClick={() => props.onClose()}>
            Cancel
          </button>
          <button class="action-button" onClick={handleSave}>
            OK
          </button>
        </div>
      </div>
    </div>
  );
};
