import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import { iconThemeStore } from "../../stores/icon-theme-store";
import { ICON_THEME_ENTRIES, getResolvedIconTheme } from "../../lib/icon-themes/icon-theme-registry";
import { applyIconTheme } from "../../lib/icon-themes/apply-icon-theme";

type IconThemePickerProps = {
  onClose: () => void;
};

export const IconThemePicker = (props: IconThemePickerProps) => {
  const { setIconTheme } = iconThemeStore.getState();
  const originalTheme = iconThemeStore.getState().currentIconTheme;
  const initialIndex = (() => {
    const idx = ICON_THEME_ENTRIES.findIndex((t) => t.id === originalTheme.id);
    return idx >= 0 ? idx : 0;
  })();

  const [search, setSearch] = createSignal("");
  const [activeIndex, setActiveIndex] = createSignal(initialIndex);
  let listRef: HTMLDivElement | undefined;
  let inputRef: HTMLInputElement | undefined;
  let isKeyboardNav = false;

  const filtered = createMemo(() => {
    const query = search();
    if (!query) return ICON_THEME_ENTRIES;
    const lower = query.toLowerCase();
    return ICON_THEME_ENTRIES.filter((e) => e.label.toLowerCase().includes(lower));
  });

  const previewTheme = (id: string) => {
    const resolved = getResolvedIconTheme(id);
    if (resolved) applyIconTheme(resolved);
  };

  const confirmTheme = (id: string) => {
    setIconTheme(id);
    props.onClose();
  };

  const cancel = () => {
    applyIconTheme(originalTheme);
    props.onClose();
  };

  createEffect(
    () => {
      const idx = activeIndex();
      const list = filtered();
      return idx >= 0 && idx < list.length ? list[idx].id : null;
    },
    (id) => {
      if (id) previewTheme(id);
    }
  );

  createEffect(
    () => activeIndex(),
    (idx) => {
      if (idx < 0 || !listRef) return;
      const items = listRef.querySelectorAll("[data-icon-theme-item]");
      items[idx]?.scrollIntoView({ block: "nearest" });
    }
  );

  createEffect(
    () => search(),
    () => {
      setActiveIndex(filtered().length > 0 ? 0 : -1);
    },
    { defer: true }
  );

  onSettled(() => {
    inputRef?.focus();
  });

  const handleKeyDown = (e: KeyboardEvent) => {
    const list = filtered();
    if (e.key === "ArrowDown") {
      e.preventDefault();
      isKeyboardNav = true;
      setActiveIndex((prev) => (list.length > 0 ? (prev + 1) % list.length : -1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      isKeyboardNav = true;
      setActiveIndex((prev) => (list.length > 0 ? (prev - 1 + list.length) % list.length : -1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const idx = activeIndex();
      if (idx >= 0 && idx < list.length) {
        confirmTheme(list[idx].id);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    }
  };

  return (
    <div class="theme-picker-overlay" onMouseDown={cancel}>
      <div class="theme-picker" onMouseDown={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <input
          ref={(el) => {
            inputRef = el;
          }}
          class="theme-picker-input"
          type="search"
          placeholder="Select File Icon Theme"
          autocomplete="off"
          autocorrect="off"
          autocapitalize="off"
          spellcheck={false}
          data-form-type="other"
          value={search()}
          onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) => setSearch(e.currentTarget.value)}
        />
        <div
          class="theme-picker-list"
          ref={(el) => {
            listRef = el;
          }}
          onMouseMove={() => {
            isKeyboardNav = false;
          }}
        >
          <For each={filtered()} keyed={(theme) => theme.id}>
            {(theme, idx) => (
              <div
                data-icon-theme-item
                class={["theme-picker-item", { active: idx() === activeIndex() }]}
                onMouseEnter={() => {
                  if (!isKeyboardNav) setActiveIndex(idx());
                }}
                onClick={() => confirmTheme(theme().id)}
              >
                <span class="theme-picker-item-label">{theme().label}</span>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};
