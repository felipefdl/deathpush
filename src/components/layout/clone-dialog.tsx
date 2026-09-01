import { createSignal, onSettled } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { repositoryStore } from "../../stores/repository-store";
import { addRecentProject } from "../../lib/recent-projects";
import * as commands from "../../lib/tauri-commands";
import { beginRepositorySession } from "../../hooks/use-repository-events";

type CloneDialogProps = {
  onClose: () => void;
};

export const CloneDialog = (props: CloneDialogProps) => {
  const [url, setUrl] = createSignal("");
  const [directory, setDirectory] = createSignal("");
  const [cloning, setCloning] = createSignal(false);
  const { setIdentity, setError } = repositoryStore.getState();
  let inputRef: HTMLInputElement | undefined;
  let overlayRef: HTMLDivElement | undefined;

  onSettled(() => {
    inputRef?.focus();
  });

  const handleBrowse = async () => {
    const selected = await open({ directory: true, title: "Choose directory to clone into" });
    if (selected) {
      setDirectory(selected);
    }
  };

  const handleClone = async () => {
    const urlValue = url().trim();
    const directoryValue = directory().trim();
    if (!urlValue || !directoryValue) return;
    const repoName =
      urlValue
        .split("/")
        .pop()
        ?.replace(/\.git$/, "") ?? "repo";
    const targetPath = `${directoryValue}/${repoName}`;
    setCloning(true);
    try {
      beginRepositorySession();
      const identity = await commands.cloneRepository(urlValue, targetPath);
      addRecentProject(identity.root, identity.headBranch ?? undefined);
      setIdentity(identity, { reset: false });
      void commands.getStatus().catch(() => undefined);
      props.onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setCloning(false);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.onClose();
    } else if (e.key === "Enter") {
      void handleClone();
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
        <div class="clone-dialog-title">Clone Repository</div>
        <div class="clone-dialog-field">
          <label class="clone-dialog-label">Repository URL</label>
          <input
            ref={(el) => {
              inputRef = el;
            }}
            class="clone-dialog-input"
            autocomplete="off"
            autocorrect="off"
            autocapitalize="off"
            spellcheck={false}
            data-form-type="other"
            value={url()}
            onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => setUrl(e.currentTarget.value)}
            placeholder="https://github.com/user/repo.git"
          />
        </div>
        <div class="clone-dialog-field">
          <label class="clone-dialog-label">Directory</label>
          <div class="clone-dialog-dir-row">
            <input
              class="clone-dialog-input"
              autocomplete="off"
              autocorrect="off"
              autocapitalize="off"
              spellcheck={false}
              data-form-type="other"
              value={directory()}
              onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => setDirectory(e.currentTarget.value)}
              placeholder="Select a directory..."
            />
            <button class="clone-dialog-browse" onClick={handleBrowse}>
              <span class="codicon codicon-folder-opened" />
            </button>
          </div>
        </div>
        <div class="clone-dialog-actions">
          <button class="action-button secondary" onClick={() => props.onClose()} disabled={cloning()}>
            Cancel
          </button>
          <button
            class="action-button"
            onClick={handleClone}
            disabled={!url().trim() || !directory().trim() || cloning()}
          >
            {cloning() ? "Cloning..." : "Clone"}
          </button>
        </div>
      </div>
    </div>
  );
};
