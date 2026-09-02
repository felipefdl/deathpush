import type { RepoOperationState } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { sendIntent } from "../../lib/session-client";

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
  const actions = useStore(repositoryStore, (s) => s.actions);
  const { setError, startOperation, endOperation } = repositoryStore.getState();

  const label = () => LABELS[props.operationState] ?? "Operation in progress";
  const operation = () => actions()?.operation;

  const run = async (type: "operationContinue" | "operationAbort" | "operationSkip") => {
    startOperation("lifecycle");
    try {
      await sendIntent({ type });
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
        {operation()?.continue ? (
          <button class="merge-banner-btn" onClick={() => void run("operationContinue")} title="Continue">
            Continue
          </button>
        ) : null}
        {operation()?.skip ? (
          <button class="merge-banner-btn" onClick={() => void run("operationSkip")} title="Skip">
            Skip
          </button>
        ) : null}
        {operation()?.abort ? (
          <button
            class="merge-banner-btn merge-banner-btn-danger"
            onClick={() => void run("operationAbort")}
            title="Abort"
          >
            Abort
          </button>
        ) : null}
      </div>
    </div>
  );
};
