import { useState, useCallback, useRef, useEffect, type KeyboardEvent } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";
import { Spinner } from "../ui/spinner";

const IS_MAC = navigator.platform.toUpperCase().includes("MAC");

export const CommitInput = () => {
  const [message, setMessage] = useState("");
  const [showDropdown, setShowDropdown] = useState(false);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  const dropdownRef = useRef<HTMLDivElement>(null);
  const { status, setStatus, setError, startOperation, endOperation, operations, amendMode, setAmendMode } =
    useRepositoryStore();

  const hasStaged = status?.groups.some((g) => g.kind === "index") ?? false;
  const hasChanges = status?.groups.some((g) => g.kind !== "index") ?? false;
  const isCommitting = operations.has("commit");
  const branch = status?.headBranch ?? "HEAD";

  useEffect(() => {
    if (!amendMode) return;
    const loadMessage = async () => {
      try {
        const lastMsg = await commands.getLastCommitMessage();
        setMessage(lastMsg);
      } catch (err) {
        setError(String(err));
        setAmendMode(false);
      }
    };
    loadMessage();
  }, [amendMode, setError, setAmendMode]);

  // Close dropdown on outside click
  useEffect(() => {
    if (!showDropdown) return;
    const handleClick = (e: MouseEvent) => {
      if (dropdownRef.current && !dropdownRef.current.contains(e.target as Node)) {
        setShowDropdown(false);
      }
    };
    document.addEventListener("mousedown", handleClick);
    return () => document.removeEventListener("mousedown", handleClick);
  }, [showDropdown]);

  // Close dropdown on Escape
  useEffect(() => {
    if (!showDropdown) return;
    const handleKey = (e: globalThis.KeyboardEvent) => {
      if (e.key === "Escape") setShowDropdown(false);
    };
    document.addEventListener("keydown", handleKey);
    return () => document.removeEventListener("keydown", handleKey);
  }, [showDropdown]);

  const doCommit = useCallback(async (amend: boolean): Promise<boolean> => {
    if (!message.trim() || isCommitting) return false;
    startOperation("commit");
    try {
      if (!hasStaged && hasChanges) {
        await commands.stageAll();
      }
      const newStatus = await commands.commitChanges(message.trim(), amend);
      setStatus(newStatus);
      setMessage("");
      if (amend) setAmendMode(false);
      return true;
    } catch (err) {
      setError(String(err));
      return false;
    } finally {
      endOperation("commit");
    }
  }, [message, isCommitting, hasStaged, hasChanges, setStatus, setError, startOperation, endOperation, setAmendMode]);

  const handleCommit = useCallback(() => {
    doCommit(amendMode);
  }, [doCommit, amendMode]);

  const autoResize = useCallback(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, []);

  useEffect(() => {
    autoResize();
  }, [message, autoResize]);

  const handleKeyDown = useCallback((e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
      e.preventDefault();
      handleCommit();
    }
  }, [handleCommit]);

  const handleAmendCommit = useCallback(() => {
    setShowDropdown(false);
    setAmendMode(true);
  }, [setAmendMode]);

  const handleCommitAndPush = useCallback(async () => {
    setShowDropdown(false);
    const ok = await doCommit(amendMode);
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
  }, [doCommit, amendMode, startOperation, endOperation, setError]);

  const handleCommitAndSync = useCallback(async () => {
    setShowDropdown(false);
    const ok = await doCommit(amendMode);
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
  }, [doCommit, amendMode, startOperation, endOperation, setError]);

  if (!status) return null;

  const canCommit = message.trim() && !isCommitting && (hasStaged || hasChanges);
  const commitLabel = amendMode
    ? (hasStaged ? "Amend" : hasChanges ? "Amend All" : "Amend")
    : (hasStaged ? "Commit" : hasChanges ? "Commit All" : "Commit");

  const placeholder = `Message (${IS_MAC ? "\u2318" : "Ctrl"}+Enter to commit on "${branch}")`;

  return (
    <div className="commit-section">
      <div className="commit-input-wrapper">
        <textarea
          ref={textareaRef}
          className="commit-input"
          value={message}
          onChange={(e) => { setMessage(e.target.value); autoResize(); }}
          onKeyDown={handleKeyDown}
          placeholder={placeholder}
          rows={1}
          autoCapitalize="off"
          autoCorrect="off"
          autoComplete="off"
          spellCheck={false}
        />
      </div>
      <div className="commit-actions">
        <div className="commit-dropdown-wrapper" ref={dropdownRef}>
          <div className="commit-button-group">
            <button
              className="action-button"
              onClick={handleCommit}
              disabled={!canCommit}
              title={amendMode ? "Amend staged changes" : "Commit staged changes"}
            >
              {isCommitting ? <Spinner /> : <span className="codicon codicon-check" />}
              {commitLabel}
            </button>
            <button
              className="commit-dropdown-toggle"
              onClick={() => setShowDropdown((v) => !v)}
              disabled={!canCommit}
              title="More commit options"
            >
              <span className="codicon codicon-chevron-down" />
            </button>
          </div>
          {showDropdown && (
            <div className="commit-dropdown">
              <div className="commit-dropdown-item" onClick={() => { setShowDropdown(false); handleCommit(); }}>
                Commit
              </div>
              <div className="commit-dropdown-item" onClick={handleAmendCommit}>
                Commit (Amend)
              </div>
              <div className="commit-dropdown-separator" />
              <div className="commit-dropdown-item" onClick={handleCommitAndPush}>
                Commit & Push
              </div>
              <div className="commit-dropdown-item" onClick={handleCommitAndSync}>
                Commit & Sync
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
