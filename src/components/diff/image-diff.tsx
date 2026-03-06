import { useEffect, useState } from "react";

interface ImageDiffProps {
  original: string;
  modified: string;
}

interface ImageMeta {
  width: number;
  height: number;
  size: number;
}

const getBase64Size = (dataUri: string): number => {
  const base64 = dataUri.split(",")[1] ?? "";
  return Math.floor(base64.length * 3 / 4);
};

const formatSize = (bytes: number): string => {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
};

const useImageMeta = (src: string): ImageMeta | null => {
  const [meta, setMeta] = useState<ImageMeta | null>(null);

  useEffect(() => {
    if (!src) {
      setMeta(null);
      return;
    }
    const img = new Image();
    img.onload = () => {
      setMeta({
        width: img.naturalWidth,
        height: img.naturalHeight,
        size: getBase64Size(src),
      });
    };
    img.src = src;
  }, [src]);

  return meta;
};

export const ImageDiff = ({ original, modified }: ImageDiffProps) => {
  const originalMeta = useImageMeta(original);
  const modifiedMeta = useImageMeta(modified);

  return (
    <div className="image-diff">
      <div className="image-diff-panel">
        <div className="image-diff-label">Original</div>
        {original ? (
          <>
            <div className="image-diff-container">
              <img src={original} alt="Original" />
            </div>
            {originalMeta && (
              <div className="image-diff-meta">
                {originalMeta.width} x {originalMeta.height} - {formatSize(originalMeta.size)}
              </div>
            )}
          </>
        ) : (
          <div className="image-diff-empty">New file</div>
        )}
      </div>
      <div className="image-diff-panel">
        <div className="image-diff-label">Modified</div>
        {modified ? (
          <>
            <div className="image-diff-container">
              <img src={modified} alt="Modified" />
            </div>
            {modifiedMeta && (
              <div className="image-diff-meta">
                {modifiedMeta.width} x {modifiedMeta.height} - {formatSize(modifiedMeta.size)}
              </div>
            )}
          </>
        ) : (
          <div className="image-diff-empty">Deleted</div>
        )}
      </div>
    </div>
  );
};
