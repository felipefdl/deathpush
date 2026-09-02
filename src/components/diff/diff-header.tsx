import { createEffect } from "solid-js";
import { acceptedBlame, sendIntent } from "../../lib/session-client";
import { useStore } from "../../lib/use-store";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { settingsStore } from "../../stores/settings-store";


export const DiffHeader = (props: { isDirty?: boolean }) => {
  const selectedFile = useStore(repositoryStore, (s) => s.selectedFile);
  const isDiffDirty = useStore(repositoryStore, (s) => s.isDiffDirty);
  const { setBlame } = repositoryStore.getState();
  const blameEnabled = useStore(settingsStore, (s) => s.settings.git.blame);
  const diffLayout = useStore(settingsStore, (s) => s.settings.diff.layout);
  const { updateDiff } = settingsStore.getState();
  const { setMainView } = layoutStore.getState();
  let fetchedPath: string | null = null;

  createEffect(
    () => [blameEnabled(), selectedFile()?.path, selectedFile()?.staged, isDiffDirty()] as const,
    ([enabled, path, staged, dirty]) => {
      if (!enabled || !path || staged || dirty) {
        fetchedPath = null;
        setBlame(null);
        return;
      }
      if (fetchedPath === path) return;
      fetchedPath = path;
      const requestGeneration = repositoryStore.getState().sessionGeneration;
      const requestRoot = repositoryStore.getState().status?.root;
      void sendIntent({ type: "openBlame", path })
        .then((result) => {
          const current = repositoryStore.getState();
          if (current.sessionGeneration !== requestGeneration) return;
          if (requestRoot !== undefined && current.status?.root !== requestRoot) return;
          if (!acceptedBlame(result)) return;
          setBlame(result.payload);
        })

    }
  );

  const handleShowFileHistory = () => {
    const file = selectedFile();
    if (!file) return;
    setMainView("history");
    window.dispatchEvent(new CustomEvent("deathpush:file-history", { detail: { path: file.path } }));
  };

  return (
    <>
      {selectedFile() && (
        <div class="diff-header">
          <span class="diff-header-path" title={selectedFile()!.path}>
            {selectedFile()!.path.split("/").pop() ?? selectedFile()!.path}
            {props.isDirty && <span class="dirty-indicator"> *</span>}
            <span class="diff-header-label">
              {" "}
              ({selectedFile()!.groupKind === "merge" ? "Merge" : selectedFile()!.staged ? "Staged" : "Working Tree"})
            </span>
          </span>
          <div class="diff-header-actions">
            <button class="scm-toolbar-button" onClick={handleShowFileHistory} title="Show File History">
              <span class="codicon codicon-history" />
            </button>
            <button
              class="scm-toolbar-button"
              onClick={() => updateDiff({ layout: diffLayout() === "inline" ? "sideBySide" : "inline" })}
              title={diffLayout() === "inline" ? "Switch to side by side" : "Switch to inline"}
            >
              <span class={`codicon ${diffLayout() === "inline" ? "codicon-split-horizontal" : "codicon-list-flat"}`} />
            </button>
          </div>
        </div>
      )}
    </>
  );
};
