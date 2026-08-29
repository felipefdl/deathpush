import { createEffect, createSignal } from "solid-js";
import type { JSX } from "@solidjs/web";

type ImageDiffProps = {
  original: string;
  modified: string;
};

type ImageMeta = {
  width: number;
  height: number;
  size: number;
};

const getBase64Size = (dataUri: string): number => {
  const base64 = dataUri.split(",")[1] ?? "";
  return Math.floor((base64.length * 3) / 4);
};

const formatSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const useImageMeta = (src: () => string): (() => ImageMeta | null) => {
  const [meta, setMeta] = createSignal<ImageMeta | null>(null);

  createEffect(
    () => src(),
    (value) => {
      if (!value) {
        setMeta(null);
        return;
      }
      const img = new Image();
      img.onload = () => {
        setMeta({
          width: img.naturalWidth,
          height: img.naturalHeight,
          size: getBase64Size(value),
        });
      };
      img.src = value;
    }
  );

  return meta;
};

export const ImageDiff = (props: ImageDiffProps) => {
  const originalMeta = useImageMeta(() => props.original);
  const modifiedMeta = useImageMeta(() => props.modified);

  const originalPanel = (): JSX.Element =>
    props.original ? (
      <>
        <div class="image-diff-container">
          <img src={props.original} alt="Original" />
        </div>
        {originalMeta() && (
          <div class="image-diff-meta">
            {originalMeta()!.width} x {originalMeta()!.height} - {formatSize(originalMeta()!.size)}
          </div>
        )}
      </>
    ) : (
      <div class="image-diff-empty">New file</div>
    );

  const modifiedPanel = (): JSX.Element =>
    props.modified ? (
      <>
        <div class="image-diff-container">
          <img src={props.modified} alt="Modified" />
        </div>
        {modifiedMeta() && (
          <div class="image-diff-meta">
            {modifiedMeta()!.width} x {modifiedMeta()!.height} - {formatSize(modifiedMeta()!.size)}
          </div>
        )}
      </>
    ) : (
      <div class="image-diff-empty">Deleted</div>
    );

  return (
    <div class="image-diff">
      <div class="image-diff-panel">
        <div class="image-diff-label">Original</div>
        {originalPanel()}
      </div>
      <div class="image-diff-panel">
        <div class="image-diff-label">Modified</div>
        {modifiedPanel()}
      </div>
    </div>
  );
};
