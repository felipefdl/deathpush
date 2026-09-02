import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import * as commands from "../../lib/tauri-commands";
import { PierreFileDiff } from "../pierre/pierre-file-diff";
import { PierreUnresolved, shouldMountMergePane } from "../pierre/pierre-unresolved";
import { DiffHeader } from "./diff-header";
import { EmptyState } from "./empty-state";
import { ImageDiff } from "./image-diff";

const openSelectedInEditor = async (): Promise<void> => {
  const file = repositoryStore.getState().selectedFile;
  if (!file) return;
  try {
    await commands.openInEditor(file.path);
  } catch (error) {
    repositoryStore.getState().setError(String(error));
  }
};

const NonPierreMessage = (props: { fileType: "binary" | "large" }) => (
  <div class="file-viewer-message">
    <span
      class={`codicon ${props.fileType === "large" ? "codicon-warning" : "codicon-file-binary"}`}
      style={{ "font-size": "32px", opacity: 0.4 }}
    />
    <p>{props.fileType === "large" ? "File is too large to display (over 5 MB)" : "Binary file cannot be displayed"}</p>
    <button
      class="action-button"
      style={{ width: "auto", padding: "0 12px" }}
      onClick={() => void openSelectedInEditor()}
    >
      Open in External Editor
    </button>
  </div>
);
export const shouldMountTextPierre = (
  selectedFile: { path: string; groupKind: string } | null,
  selectedLoadId: number,
  diff: { path: string } | null,
  diffLoadId: number | null
): boolean =>
  selectedFile !== null &&
  selectedFile.groupKind !== "merge" &&
  diff !== null &&
  diff.path === selectedFile.path &&
  diffLoadId === selectedLoadId;

export const DiffViewer = () => {
  const diff = useStore(repositoryStore, (s) => s.diff);
  const selectedFile = useStore(repositoryStore, (s) => s.selectedFile);
  const selectedLoadId = useStore(repositoryStore, (s) => s.selectedLoadId);
  const diffLoadId = useStore(repositoryStore, (s) => s.diffLoadId);
  const isDiffDirty = useStore(repositoryStore, (s) => s.isDiffDirty);

  return (
    <>
      {!diff() || !selectedFile() ? (
        <EmptyState />
      ) : diff()!.fileType === "image" ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <ImageDiff original={diff()!.original} modified={diff()!.modified} />
        </div>
      ) : diff()!.fileType === "large" || diff()!.fileType === "binary" ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <NonPierreMessage fileType={diff()!.fileType === "large" ? "large" : "binary"} />
        </div>
      ) : shouldMountMergePane(selectedFile(), selectedLoadId(), diff(), diffLoadId()) ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <div class="diff-editor-container">
            <PierreUnresolved path={selectedFile()!.path} contents={diff()!.modified} />
          </div>
        </div>
      ) : selectedFile()!.groupKind === "merge" ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
        </div>
      ) : shouldMountTextPierre(selectedFile(), selectedLoadId(), diff(), diffLoadId()) ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <div class="diff-editor-container">
            <PierreFileDiff
              path={selectedFile()!.path}
              staged={selectedFile()!.staged}
              groupKind={selectedFile()!.groupKind}
              loadId={selectedLoadId()}
            />
          </div>
        </div>
      ) : (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
        </div>
      )}
    </>
  );
};
