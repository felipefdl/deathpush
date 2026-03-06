import { useCallback } from "react";
import type { RepoOperationState } from "../../lib/git-types";
import { useRepositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";

interface MergeBannerProps {
  operationState: RepoOperationState;
}

const LABELS: Record<string, string> = {
  merging: "Merge in progress",
  rebasing: "Rebase in progress",
  cherryPicking: "Cherry-pick in progress",
  reverting: "Revert in progress",
};

export const MergeBanner = ({ operationState }: MergeBannerProps) => {
  const { setStatus, setError, startOperation, endOperation } = useRepositoryStore();

  const label = LABELS[operationState] ?? "Operation in progress";
  const isMerge = operationState === "merging";
  const isRebase = operationState === "rebasing";

  const handleContinue = useCallback(async () => {
    startOperation("lifecycle");
    try {
      const status = isMerge
        ? await commands.mergeContinue()
        : isRebase
          ? await commands.rebaseContinue()
          : await commands.mergeContinue();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  }, [isMerge, isRebase, setStatus, setError, startOperation, endOperation]);

  const handleAbort = useCallback(async () => {
    startOperation("lifecycle");
    try {
      const status = isMerge
        ? await commands.mergeAbort()
        : isRebase
          ? await commands.rebaseAbort()
          : await commands.mergeAbort();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  }, [isMerge, isRebase, setStatus, setError, startOperation, endOperation]);

  const handleSkip = useCallback(async () => {
    if (!isRebase) return;
    startOperation("lifecycle");
    try {
      const status = await commands.rebaseSkip();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  }, [isRebase, setStatus, setError, startOperation, endOperation]);

  return (
    <div className="merge-banner">
      <span className="codicon codicon-warning merge-banner-icon" />
      <span className="merge-banner-label">{label}</span>
      <div className="merge-banner-actions">
        <button className="merge-banner-btn" onClick={handleContinue} title="Continue">
          Continue
        </button>
        {isRebase && (
          <button className="merge-banner-btn" onClick={handleSkip} title="Skip">
            Skip
          </button>
        )}
        <button className="merge-banner-btn merge-banner-btn-danger" onClick={handleAbort} title="Abort">
          Abort
        </button>
      </div>
    </div>
  );
};
