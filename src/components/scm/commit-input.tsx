import { createEffect, createSignal } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import * as commands from "../../lib/tauri-commands";
import { Spinner } from "../ui/spinner";
import { IS_MACOS } from "../../lib/platform";

export const CommitInput = () => {
  const [message, setMessage] = createSignal("");
  const [showDropdown, setShowDropdown] = createSignal(false);
  let textareaRef: HTMLTextAreaElement | undefined;
  let dropdownRef: HTMLDivElement | undefined;
  const status = useStore(repositoryStore, (s) => s.status);
  const operations = useStore(repositoryStore, (s) => s.operations);
  const amendMode = useStore(repositoryStore, (s) => s.amendMode);
  const { setError, startOperation, endOperation, setAmendMode } = repositoryStore.getState();

  const hasStaged = () => status()?.groups.some((g) => g.kind === "index") ?? false;
  const hasChanges = () => status()?.groups.some((g) => g.kind !== "index") ?? false;
  const isCommitting = () => operations().has("commit");
  const branch = () => status()?.headBranch ?? "HEAD";
  const canCommit = () => message().trim() && !isCommitting() && (hasStaged() || hasChanges());
  const commitLabel = () =>
    amendMode()
      ? hasStaged()
        ? "Amend"
        : hasChanges()
          ? "Amend All"
          : "Amend"
      : hasStaged()
        ? "Commit"
        : hasChanges()
          ? "Commit All"
          : "Commit";
  const placeholderHint = () => `${IS_MACOS ? "\u2318" : "Ctrl"}+Enter to commit on "${branch()}"`;

  createEffect(
    () => amendMode(),
    (amend) => {
      if (!amend) return;
      const loadMessage = async () => {
        try {
          const lastMsg = await commands.getLastCommitMessage();
          setMessage(lastMsg);
        } catch (err) {
          setError(String(err));
          setAmendMode(false);
        }
      };
      void loadMessage();
    }
  );

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

  const doCommit = async (amend: boolean): Promise<boolean> => {
    if (!message().trim() || isCommitting()) return false;
    startOperation("commit");
    try {
      if (!hasStaged() && hasChanges()) {
        await commands.stageAll();
      }
      await commands.commitChanges(message().trim(), amend);
      setMessage("");
      if (amend) setAmendMode(false);
      return true;
    } catch (err) {
      setError(String(err));
      return false;
    } finally {
      endOperation("commit");
    }
  };

  const handleCommit = () => {
    void doCommit(amendMode());
  };

  const autoResize = () => {
    const el = textareaRef;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  };

  createEffect(
    () => message(),
    () => {
      autoResize();
    }
  );

  const handleKeyDown = (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleCommit();
    }
  };

  const handleAmendCommit = () => {
    setShowDropdown(false);
    setAmendMode(true);
  };

  const handleCommitAndPush = async () => {
    setShowDropdown(false);
    const ok = await doCommit(amendMode());
    if (ok) {
      try {
        startOperation("push");
        await commands.push();
      } catch (err) {
        setError(String(err));
      } finally {
        endOperation("push");
      }
    }
  };

  const handleCommitAndSync = async () => {
    setShowDropdown(false);
    const ok = await doCommit(amendMode());
    if (ok) {
      try {
        startOperation("sync");
        await commands.pull();
        await commands.push();
      } catch (err) {
        setError(String(err));
      } finally {
        endOperation("sync");
      }
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
              value={message()}
              onInput={(e) => {
                setMessage(e.currentTarget.value);
                autoResize();
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
                  <div class="commit-dropdown-item" onClick={handleCommitAndPush}>
                    Commit & Push
                  </div>
                  <div class="commit-dropdown-item" onClick={handleCommitAndSync}>
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
