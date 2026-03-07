import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useThemeStore } from "../../stores/theme-store";
import { THEME_ENTRIES } from "../../lib/themes/theme-registry";
import { applyTheme } from "../../lib/themes/apply-theme";
import { getResolvedTheme } from "../../lib/themes/theme-registry";
import type { ThemeEntry, ThemeKind } from "../../lib/themes/theme-types";

interface ThemePickerProps {
  onClose: () => void;
}

interface GroupedThemes {
  kind: ThemeKind;
  label: string;
  themes: ThemeEntry[];
}

const GROUP_LABELS: Record<ThemeKind, string> = {
  dark: "dark themes",
  light: "light themes",
  "hc-dark": "high contrast",
  "hc-light": "high contrast",
};

const getGroupOrder = (): ThemeKind[] => {
  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  return prefersDark
    ? ["dark", "light", "hc-dark", "hc-light"]
    : ["light", "dark", "hc-dark", "hc-light"];
};

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

  const hcDark = groups.get("hc-dark") ?? [];
  const hcLight = groups.get("hc-light") ?? [];
  const hcCombined = [...hcDark, ...hcLight];
  hcCombined.sort((a, b) => a.label.localeCompare(b.label));

  return order
    .filter((kind, _i, arr) => {
      if (kind === "hc-light" && arr.includes("hc-dark")) return false;
      return true;
    })
    .map((kind) => ({
      kind,
      label: GROUP_LABELS[kind],
      themes: kind === "hc-dark" ? hcCombined : (groups.get(kind) ?? []),
    }))
    .filter((g) => g.themes.length > 0);
};

export const ThemePicker = ({ onClose }: ThemePickerProps) => {
  const { currentTheme, setTheme } = useThemeStore();
  const originalThemeRef = useRef(currentTheme);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState(-1);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const isKeyboardNavRef = useRef(false);

  const filtered = useMemo(() => {
    if (!search) return THEME_ENTRIES;
    const lower = search.toLowerCase();
    return THEME_ENTRIES.filter((e) => e.label.toLowerCase().includes(lower));
  }, [search]);

  const groups = useMemo(() => groupThemes(filtered), [filtered]);

  const flatList = useMemo(() => groups.flatMap((g) => g.themes), [groups]);

  useEffect(() => {
    const idx = flatList.findIndex((t) => t.id === currentTheme.id);
    setActiveIndex(idx >= 0 ? idx : 0);
  }, []);

  const previewTheme = useCallback((id: string) => {
    const resolved = getResolvedTheme(id);
    if (resolved) applyTheme(resolved);
  }, []);

  const confirmTheme = useCallback((id: string) => {
    setTheme(id);
    onClose();
  }, [setTheme, onClose]);

  const cancel = useCallback(() => {
    applyTheme(originalThemeRef.current);
    onClose();
  }, [onClose]);

  useEffect(() => {
    if (activeIndex >= 0 && activeIndex < flatList.length) {
      previewTheme(flatList[activeIndex].id);
    }
  }, [activeIndex, flatList, previewTheme]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (activeIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll("[data-theme-item]");
    items[activeIndex]?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      isKeyboardNavRef.current = true;
      setActiveIndex((prev) => (prev + 1) % flatList.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      isKeyboardNavRef.current = true;
      setActiveIndex((prev) => (prev - 1 + flatList.length) % flatList.length);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (activeIndex >= 0 && activeIndex < flatList.length) {
        confirmTheme(flatList[activeIndex].id);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    }
  }, [activeIndex, flatList, confirmTheme, cancel]);

  useEffect(() => {
    setActiveIndex(flatList.length > 0 ? 0 : -1);
  }, [search]);

  const groupOffsets = useMemo(() => {
    const offsets: number[] = [];
    let offset = 0;
    for (const group of groups) {
      offsets.push(offset);
      offset += group.themes.length;
    }
    return offsets;
  }, [groups]);

  return (
    <div className="theme-picker-overlay" onMouseDown={cancel}>
      <div className="theme-picker" onMouseDown={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <input
          ref={inputRef}
          className="theme-picker-input"
          type="search"
          placeholder="Select Color Theme"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          data-form-type="other"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <div className="theme-picker-list" ref={listRef} onMouseMove={() => { isKeyboardNavRef.current = false; }}>
          {groups.map((group, gi) => (
            <div key={group.kind}>
              <div className={`theme-picker-separator${gi === 0 ? " first" : ""}`}>
                <span className="theme-picker-group-label">{group.label}</span>
              </div>
              {group.themes.map((theme, ti) => {
                const idx = groupOffsets[gi] + ti;
                return (
                  <div
                    key={theme.id}
                    data-theme-item
                    className={`theme-picker-item${idx === activeIndex ? " active" : ""}`}
                    onMouseEnter={() => { if (!isKeyboardNavRef.current) setActiveIndex(idx); }}
                    onClick={() => confirmTheme(theme.id)}
                  >
                    <span className="theme-picker-item-label">{theme.label}</span>
                    {theme.description && (
                      <span className="theme-picker-item-description">{theme.description}</span>
                    )}
                  </div>
                );
              })}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
