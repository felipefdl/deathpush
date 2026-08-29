import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { isNonPierreFileType } from "../pierre/pierre-file-diff";
import { PierreFileDiff } from "../pierre/pierre-file-diff";
import { PierreUnresolved } from "../pierre/pierre-unresolved";
import { DiffHeader } from "./diff-header";
import { EmptyState } from "./empty-state";
import { ImageDiff } from "./image-diff";

export const DiffViewer = () => {
  const diff = useStore(repositoryStore, (s) => s.diff);
  const selectedFile = useStore(repositoryStore, (s) => s.selectedFile);
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
      ) : isNonPierreFileType(diff()!.fileType) ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
        </div>
      ) : selectedFile()!.groupKind === "merge" ? (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <div class="diff-editor-container">
            <PierreUnresolved path={selectedFile()!.path} contents={diff()!.modified} />
          </div>
        </div>
      ) : (
        <div class="diff-viewer">
          <DiffHeader isDirty={isDiffDirty()} />
          <div class="diff-editor-container">
            <PierreFileDiff
              path={selectedFile()!.path}
              staged={selectedFile()!.staged}
              groupKind={selectedFile()!.groupKind}
            />
          </div>
        </div>
      )}
    </>
  );
};
