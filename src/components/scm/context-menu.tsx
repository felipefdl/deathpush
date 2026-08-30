import { For, onSettled } from "solid-js";
import { Portal } from "@solidjs/web";

export type ContextMenuItem = {
  label: string;
  icon?: string;
  action: () => void;
  separator?: boolean;
  disabled?: boolean;
};

type ContextMenuProps = {
  x: number;
  y: number;
  items: ContextMenuItem[];
  onClose: () => void;
  treeContextRoot?: boolean;
};

export const ContextMenu = (props: ContextMenuProps) => {
  let menuRef: HTMLDivElement | undefined;

  onSettled(() => {
    const handleClick = (e: MouseEvent) => {
      if (menuRef && !menuRef.contains(e.target as Node)) {
        props.onClose();
      }
    };
    const handleEscape = (e: KeyboardEvent) => {
      if (e.key === "Escape") props.onClose();
    };
    document.addEventListener("mousedown", handleClick);
    document.addEventListener("keydown", handleEscape);
    return () => {
      document.removeEventListener("mousedown", handleClick);
      document.removeEventListener("keydown", handleEscape);
    };
  });

  const adjustedX = () => Math.min(props.x, window.innerWidth - 200);
  const adjustedY = () => Math.min(props.y, window.innerHeight - props.items.length * 28 - 8);

  return (
    <Portal>
      <div
        class="context-menu"
        data-file-tree-context-menu-root={props.treeContextRoot ? "true" : undefined}
        ref={(el) => {
          menuRef = el;
        }}
        style={{ left: `${adjustedX()}px`, top: `${adjustedY()}px` }}
      >
        <For each={props.items} keyed={false}>
          {(item) =>
            item().separator ? (
              <div class="context-menu-separator" />
            ) : (
              <div
                class={`context-menu-item ${item().disabled ? "disabled" : ""}`}
                onClick={() => {
                  const current = item();
                  if (!current.disabled) {
                    current.action();
                    props.onClose();
                  }
                }}
              >
                {item().icon && (
                  <span
                    class={`codicon codicon-${item().icon}`}
                    style={{ "margin-right": "8px", "font-size": "14px" }}
                  />
                )}
                <span>{item().label}</span>
              </div>
            )
          }
        </For>
      </div>
    </Portal>
  );
};
