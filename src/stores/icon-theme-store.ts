import { create } from "zustand";
import type { ResolvedIconTheme } from "../lib/icon-themes/icon-theme-types";
import { getResolvedIconTheme, DEFAULT_ICON_THEME_ID } from "../lib/icon-themes/icon-theme-registry";
import { applyIconTheme } from "../lib/icon-themes/apply-icon-theme";

const STORAGE_KEY = "deathpush:iconTheme";

const resolveInitialIconTheme = (): ResolvedIconTheme => {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) {
    const theme = getResolvedIconTheme(stored);
    if (theme) return theme;
  }
  return getResolvedIconTheme(DEFAULT_ICON_THEME_ID)!;
};

interface IconThemeState {
  currentIconTheme: ResolvedIconTheme;
  setIconTheme: (id: string) => void;
}

export const useIconThemeStore = create<IconThemeState>((set) => ({
  currentIconTheme: resolveInitialIconTheme(),
  setIconTheme: (id: string) => {
    const theme = getResolvedIconTheme(id);
    if (!theme) return;
    applyIconTheme(theme);
    set({ currentIconTheme: theme });
  },
}));
