import { useCallback, useRef, useEffect } from "react";
import { open as shellOpen } from "@tauri-apps/plugin-shell";

declare const __LICENSES__: { name: string; license: string; url: string; category: "npm" | "rust" | "asset" }[];

interface LicensesModalProps {
  onClose: () => void;
}

const CATEGORY_LABELS: Record<string, string> = {
  asset: "Assets",
  npm: "Frontend",
  rust: "Backend",
};

const CATEGORY_ORDER = ["asset", "npm", "rust"] as const;

export const LicensesModal = ({ onClose }: LicensesModalProps) => {
  const overlayRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, [onClose]);

  const handleOverlayClick = useCallback(
    (e: React.MouseEvent) => {
      if (e.target === overlayRef.current) onClose();
    },
    [onClose],
  );

  const grouped = new Map<string, typeof __LICENSES__>();
  for (const entry of __LICENSES__) {
    const list = grouped.get(entry.category) ?? [];
    list.push(entry);
    grouped.set(entry.category, list);
  }

  return (
    <div className="branch-picker-overlay" ref={overlayRef} onClick={handleOverlayClick}>
      <div className="licenses-modal">
        <div className="clone-dialog-title">Open Source Licenses</div>
        <div className="licenses-list">
          {CATEGORY_ORDER.map((cat) => {
            const items = grouped.get(cat);
            if (!items?.length) return null;
            return (
              <div key={cat}>
                <div className="licenses-group-title">{CATEGORY_LABELS[cat]}</div>
                {items.map((entry) => (
                  <div key={entry.name} className="license-entry">
                    <span className="license-entry-name">{entry.name}</span>
                    <span className="license-badge">{entry.license}</span>
                    {entry.url && (
                      <button
                        className="license-link"
                        onClick={() => shellOpen(entry.url)}
                        title={entry.url}
                      >
                        <span className="codicon codicon-link-external" />
                      </button>
                    )}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
        <div className="clone-dialog-actions">
          <button className="action-button secondary" onClick={onClose}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
