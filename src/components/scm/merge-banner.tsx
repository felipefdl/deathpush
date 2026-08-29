import type { RepoOperationState } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";

type MergeBannerProps = {
  operationState: RepoOperationState;
};

const LABELS: Record<string, string> = {
  merging: "Merge in progress",
  rebasing: "Rebase in progress",
  cherryPicking: "Cherry-pick in progress",
  reverting: "Revert in progress",
};

export const MergeBanner = (props: MergeBannerProps) => {
  const { setStatus, setError, startOperation, endOperation } = repositoryStore.getState();

  const label = () => LABELS[props.operationState] ?? "Operation in progress";
  const isMerge = () => props.operationState === "merging";
  const isRebase = () => props.operationState === "rebasing";

  const handleContinue = async () => {
    startOperation("lifecycle");
    try {
      const status = isMerge()
        ? await commands.mergeContinue()
        : isRebase()
          ? await commands.rebaseContinue()
          : await commands.mergeContinue();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  };

  const handleAbort = async () => {
    startOperation("lifecycle");
    try {
      const status = isMerge()
        ? await commands.mergeAbort()
        : isRebase()
          ? await commands.rebaseAbort()
          : await commands.mergeAbort();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  };

  const handleSkip = async () => {
    if (!isRebase()) return;
    startOperation("lifecycle");
    try {
      const status = await commands.rebaseSkip();
      setStatus(status);
    } catch (err) {
      setError(String(err));
    } finally {
      endOperation("lifecycle");
    }
  };

  return (
    <div class="merge-banner">
      <span class="codicon codicon-warning merge-banner-icon" />
      <span class="merge-banner-label">{label()}</span>
      <div class="merge-banner-actions">
        <button class="merge-banner-btn" onClick={handleContinue} title="Continue">
          Continue
        </button>
        {isRebase() && (
          <button class="merge-banner-btn" onClick={handleSkip} title="Skip">
            Skip
          </button>
        )}
        <button class="merge-banner-btn merge-banner-btn-danger" onClick={handleAbort} title="Abort">
          Abort
        </button>
      </div>
    </div>
  );
};
