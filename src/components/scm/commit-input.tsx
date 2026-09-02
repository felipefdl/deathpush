import { createEffect, createSignal } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { sendDestructiveIntent, sendIntent } from "../../lib/session-client";
import { Spinner } from "../ui/spinner";
import { IS_MACOS } from "../../lib/platform";

export const CommitInput = () => {
  const [showDropdown, setShowDropdown] = createSignal(false);
  let textareaRef: HTMLTextAreaElement | undefined;
  let dropdownRef: HTMLDivElement | undefined;
  const status = useStore(repositoryStore, (s) => s.status);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const amendMode = useStore(repositoryStore, (s) => s.amendMode);
  const commitMessage = useStore(repositoryStore, (s) => s.commitMessage);
  const actions = useStore(repositoryStore, (s) => s.actions);
  const { setError, startOperation, endOperation } = repositoryStore.getState();

  const isCommitting = () => operations().has("commit");
  const branch = () => status()?.headBranch ?? "HEAD";
  const canCommit = () => (actions()?.canCommit ?? false) && !isCommitting();
  const commitLabel = () => actions()?.commitLabel ?? "Commit";
  const placeholderHint = () => `${IS_MACOS ? "\u2318" : "Ctrl"}+Enter to commit on "${branch()}"`;

  createEffect(
    () => showDropdown(),
    (open) => {
      if (!open) return;
      const handleClick = (e: MouseEvent) => {
        if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
          setShowDropdown(false);
        }
      };
      const handleKey = (e: KeyboardEvent) => {
        if (e.key === "Escape") setShowDropdown(false);
      };
      document.addEventListener("mousedown", handleClick);
      document.addEventListener("keydown", handleKey);
      return () => {
        document.removeEventListener("mousedown", handleClick);
        document.removeEventListener("keydown", handleKey);
      };
    }
  );

  const autoResize = () => {
    const el = textareaRef;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  };

  createEffect(
    () => commitMessage(),
    () => {
      autoResize();
    }
  );

  const doCommit = async (): Promise<boolean> => {
    if (!canCommit()) return false;
    startOperation("commit");
    try {
      const result = await sendDestructiveIntent({ type: "commit", confirmed: !actions()?.commitDestructive });
      return result.kind === "snapshot";
    } catch (err) {
      setError(String(err));
      return false;
    } finally {
      endOperation("commit");
    }
  };

  const handleCommit = () => {
    void doCommit();
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleCommit();
    }
  };

  const handleAmendCommit = () => {
    setShowDropdown(false);
    void sendIntent({ type: "setAmend", enabled: true });
  };

  const handleCommitAndPush = async () => {
    setShowDropdown(false);
    if (!canCommit()) return;
    startOperation("commit");
    try {
      await sendDestructiveIntent({ type: "commitAndPush", confirmed: !actions()?.commitDestructive });
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("commit");
    }
  };

  const handleCommitAndSync = async () => {
    setShowDropdown(false);
    if (!canCommit()) return;
    startOperation("commit");
    try {
      await sendDestructiveIntent({ type: "commitAndSync", confirmed: !actions()?.commitDestructive });
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("commit");
    }
  };


  return (
    <>
      {status() ? (
        <div class="commit-section">
          <div class="commit-input-wrapper">
            <textarea
              ref={(el) => {
                textareaRef = el;
              }}
              class="commit-input"
              value={commitMessage()}
              onInput={(e) => {
                const message = e.currentTarget.value;
                repositoryStore.setState({ commitMessage: message });
                autoResize();
                void sendIntent({ type: "setCommitMessage", message });
              }}
              onKeyDown={handleKeyDown}
              placeholder="commit message"
              title={placeholderHint()}
              rows={2}
              autocapitalize="off"
              autocorrect="off"
              autocomplete="off"
              spellcheck={false}
            />
          </div>
          <div class="commit-actions">
            <div
              class="commit-dropdown-wrapper"
              ref={(el) => {
                dropdownRef = el;
              }}
            >
              <div class="commit-button-group">
                <button
                  class="action-button"
                  onClick={handleCommit}
                  disabled={!canCommit()}
                  title={amendMode() ? "Amend staged changes" : "Commit staged changes"}
                >
                  {isCommitting() ? <Spinner /> : <span class="codicon codicon-check" />}
                  {commitLabel()}
                </button>
                <button
                  class="commit-dropdown-toggle"
                  onClick={() => setShowDropdown((v) => !v)}
                  disabled={!canCommit()}
                  title="More commit options"
                >
                  <span class="codicon codicon-chevron-down" />
                </button>
              </div>
              {showDropdown() && (
                <div class="commit-dropdown">
                  <div
                    class="commit-dropdown-item"
                    onClick={() => {
                      setShowDropdown(false);
                      handleCommit();
                    }}
                  >
                    Commit
                  </div>
                  <div class="commit-dropdown-item" onClick={handleAmendCommit}>
                    Commit (Amend)
                  </div>
                  <div class="commit-dropdown-separator" />
                  <div class="commit-dropdown-item" onClick={() => void handleCommitAndPush()}>
                    Commit & Push
                  </div>
                  <div class="commit-dropdown-item" onClick={() => void handleCommitAndSync()}>
                    Commit & Sync
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      ) : null}
    </>
  );
};
