import { For, onSettled } from "solid-js";
import { open as shellOpen } from "@tauri-apps/plugin-shell";

declare const __LICENSES__: { name: string; license: string; url: string; category: "npm" | "rust" | "asset" }[];

type LicensesModalProps = {
  onClose: () => void;
};

const CATEGORY_LABELS: Record<string, string> = {
  asset: "Assets",
  npm: "Frontend",
  rust: "Backend",
};

const CATEGORY_ORDER = ["asset", "npm", "rust"] as const;

export const LicensesModal = (props: LicensesModalProps) => {
  let overlayRef: HTMLDivElement | undefined;

  onSettled(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") props.onClose();
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  });

  const handleOverlayClick = (e: MouseEvent) => {
    if (e.target === overlayRef) props.onClose();
  };

  const grouped = new Map<string, typeof __LICENSES__>();
  for (const entry of __LICENSES__) {
    const list = grouped.get(entry.category) ?? [];
    list.push(entry);
    grouped.set(entry.category, list);
  }

  return (
    <div
      class="branch-picker-overlay"
      ref={(el) => {
        overlayRef = el;
      }}
      onClick={handleOverlayClick}
    >
      <div class="licenses-modal">
        <div class="clone-dialog-title">Open Source Licenses</div>
        <div class="licenses-list">
          <For each={CATEGORY_ORDER} keyed>
            {(cat) => {
              const items = grouped.get(cat);
              if (!items?.length) return null;
              return (
                <div>
                  <div class="licenses-group-title">{CATEGORY_LABELS[cat]}</div>
                  <For each={items} keyed={(entry) => entry.name}>
                    {(entry) => (
                      <div class="license-entry">
                        <span class="license-entry-name">{entry().name}</span>
                        <span class="license-badge">{entry().license}</span>
                        {entry().url && (
                          <button class="license-link" onClick={() => shellOpen(entry().url)} title={entry().url}>
                            <span class="codicon codicon-link-external" />
                          </button>
                        )}
                      </div>
                    )}
                  </For>
                </div>
              );
            }}
          </For>
        </div>
        <div class="clone-dialog-actions">
          <button class="action-button secondary" onClick={() => props.onClose()}>
            Close
          </button>
        </div>
      </div>
    </div>
  );
};
