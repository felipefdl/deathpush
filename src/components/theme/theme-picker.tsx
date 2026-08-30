import { createEffect, createMemo, createSignal, For, onSettled } from "solid-js";
import { themeStore } from "../../stores/theme-store";
import { THEME_ENTRIES } from "../../lib/themes/theme-registry";
import { applyTheme } from "../../lib/themes/apply-theme";
import { getResolvedTheme } from "../../lib/themes/theme-registry";
import type { ThemeEntry, ThemeKind } from "../../lib/themes/theme-types";

type ThemePickerProps = {
  onClose: () => void;
};

type GroupedThemes = {
  kind: ThemeKind;
  label: string;
  themes: ThemeEntry[];
};

const GROUP_LABELS: Record<ThemeKind, string> = {
  dark: "dark themes",
  light: "light themes",
};

const getGroupOrder = (): ThemeKind[] =>
  window.matchMedia("(prefers-color-scheme: dark)").matches ? ["dark", "light"] : ["light", "dark"];

const groupThemes = (entries: ThemeEntry[]): GroupedThemes[] => {
  const order = getGroupOrder();
  const groups = new Map<ThemeKind, ThemeEntry[]>();

  for (const entry of entries) {
    const list = groups.get(entry.kind) ?? [];
    list.push(entry);
    groups.set(entry.kind, list);
  }

  for (const list of groups.values()) {
    list.sort((a, b) => a.label.localeCompare(b.label));
  }

  return order
    .map((kind) => ({
      kind,
      label: GROUP_LABELS[kind],
      themes: groups.get(kind) ?? [],
    }))
    .filter((group) => group.themes.length > 0);
};

export const ThemePicker = (props: ThemePickerProps) => {
  const { setTheme } = themeStore.getState();
  const originalTheme = themeStore.getState().currentTheme;
  const currentThemeId = originalTheme.id;
  const initialIndex = (() => {
    const flat = groupThemes(THEME_ENTRIES).flatMap((g) => g.themes);
    const idx = flat.findIndex((t) => t.id === currentThemeId);
    return idx >= 0 ? idx : 0;
  })();

  const [search, setSearch] = createSignal("");
  const [activeIndex, setActiveIndex] = createSignal(initialIndex);
  let listRef: HTMLDivElement | undefined;
  let inputRef: HTMLInputElement | undefined;
  let isKeyboardNav = false;
  let previewRequest = 0;

  const filtered = createMemo(() => {
    const query = search();
    if (!query) return THEME_ENTRIES;
    const lower = query.toLowerCase();
    return THEME_ENTRIES.filter((e) => e.label.toLowerCase().includes(lower));
  });

  const groups = createMemo(() => groupThemes(filtered()));
  const flatList = createMemo(() => groups().flatMap((g) => g.themes));

  const groupOffsets = createMemo(() => {
    const offsets: number[] = [];
    let offset = 0;
    for (const group of groups()) {
      offsets.push(offset);
      offset += group.themes.length;
    }
    return offsets;
  });

  const previewTheme = async (id: string): Promise<void> => {
    const request = ++previewRequest;
    const resolved = await getResolvedTheme(id);
    if (resolved && request === previewRequest) {
      themeStore.setState({ currentTheme: resolved });
      applyTheme(resolved, { transient: true });
    }
  };

  const confirmTheme = (id: string): void => {
    previewRequest += 1;
    props.onClose();
    void setTheme(id);
  };

  const cancel = (): void => {
    previewRequest += 1;
    themeStore.setState({ currentTheme: originalTheme });
    applyTheme(originalTheme);
    props.onClose();
  };

  createEffect(
    () => {
      const idx = activeIndex();
      const list = flatList();
      return idx >= 0 && idx < list.length ? list[idx].id : null;
    },
    (id) => {
      if (id && isKeyboardNav) void previewTheme(id);
    }
  );

  createEffect(
    () => activeIndex(),
    (idx) => {
      if (idx < 0 || !listRef) return;
      const items = listRef.querySelectorAll("[data-theme-item]");
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
    const list = flatList();
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
          placeholder="Select Color Theme"
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
          <For each={groups()} keyed={(group) => group.kind}>
            {(group, gi) => (
              <div>
                <div class={["theme-picker-separator", { first: gi() === 0 }]}>
                  <span class="theme-picker-group-label">{group().label}</span>
                </div>
                <For each={group().themes} keyed={(theme) => theme.id}>
                  {(theme, ti) => (
                    <div
                      data-theme-item
                      class={["theme-picker-item", { active: groupOffsets()[gi()] + ti() === activeIndex() }]}
                      onMouseEnter={() => {
                        if (!isKeyboardNav) setActiveIndex(groupOffsets()[gi()] + ti());
                      }}
                      onClick={() => confirmTheme(theme().id)}
                    >
                      <span class="theme-picker-item-label">{theme().label}</span>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>
      </div>
    </div>
  );
};
