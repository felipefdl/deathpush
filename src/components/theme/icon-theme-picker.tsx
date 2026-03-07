import { useState, useEffect, useRef, useCallback, useMemo } from "react";
import { useIconThemeStore } from "../../stores/icon-theme-store";
import { ICON_THEME_ENTRIES, getResolvedIconTheme } from "../../lib/icon-themes/icon-theme-registry";
import { applyIconTheme } from "../../lib/icon-themes/apply-icon-theme";

interface IconThemePickerProps {
  onClose: () => void;
}

export const IconThemePicker = ({ onClose }: IconThemePickerProps) => {
  const { currentIconTheme, setIconTheme } = useIconThemeStore();
  const originalThemeRef = useRef(currentIconTheme);
  const [search, setSearch] = useState("");
  const [activeIndex, setActiveIndex] = useState(-1);
  const listRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLInputElement>(null);
  const isKeyboardNavRef = useRef(false);

  const filtered = useMemo(() => {
    if (!search) return ICON_THEME_ENTRIES;
    const lower = search.toLowerCase();
    return ICON_THEME_ENTRIES.filter((e) => e.label.toLowerCase().includes(lower));
  }, [search]);

  useEffect(() => {
    const idx = filtered.findIndex((t) => t.id === currentIconTheme.id);
    setActiveIndex(idx >= 0 ? idx : 0);
  }, []);

  const previewTheme = useCallback((id: string) => {
    const resolved = getResolvedIconTheme(id);
    if (resolved) applyIconTheme(resolved);
  }, []);

  const confirmTheme = useCallback((id: string) => {
    setIconTheme(id);
    onClose();
  }, [setIconTheme, onClose]);

  const cancel = useCallback(() => {
    applyIconTheme(originalThemeRef.current);
    onClose();
  }, [onClose]);

  useEffect(() => {
    if (activeIndex >= 0 && activeIndex < filtered.length) {
      previewTheme(filtered[activeIndex].id);
    }
  }, [activeIndex, filtered, previewTheme]);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (activeIndex < 0 || !listRef.current) return;
    const items = listRef.current.querySelectorAll("[data-icon-theme-item]");
    items[activeIndex]?.scrollIntoView({ block: "nearest" });
  }, [activeIndex]);

  const handleKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      isKeyboardNavRef.current = true;
      setActiveIndex((prev) => (prev + 1) % filtered.length);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      isKeyboardNavRef.current = true;
      setActiveIndex((prev) => (prev - 1 + filtered.length) % filtered.length);
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (activeIndex >= 0 && activeIndex < filtered.length) {
        confirmTheme(filtered[activeIndex].id);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      cancel();
    }
  }, [activeIndex, filtered, confirmTheme, cancel]);

  useEffect(() => {
    setActiveIndex(filtered.length > 0 ? 0 : -1);
  }, [search]);

  return (
    <div className="theme-picker-overlay" onMouseDown={cancel}>
      <div className="theme-picker" onMouseDown={(e) => e.stopPropagation()} onKeyDown={handleKeyDown}>
        <input
          ref={inputRef}
          className="theme-picker-input"
          type="search"
          placeholder="Select File Icon Theme"
          autoComplete="off"
          autoCorrect="off"
          autoCapitalize="off"
          spellCheck={false}
          data-form-type="other"
          value={search}
          onChange={(e) => setSearch(e.target.value)}
        />
        <div className="theme-picker-list" ref={listRef} onMouseMove={() => { isKeyboardNavRef.current = false; }}>
          {filtered.map((theme, idx) => (
            <div
              key={theme.id}
              data-icon-theme-item
              className={`theme-picker-item${idx === activeIndex ? " active" : ""}`}
              onMouseEnter={() => { if (!isKeyboardNavRef.current) setActiveIndex(idx); }}
              onClick={() => confirmTheme(theme.id)}
            >
              <span className="theme-picker-item-label">{theme.label}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
