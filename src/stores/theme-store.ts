import { create } from "zustand";
import type { ResolvedTheme } from "../lib/themes/theme-types";
import { getResolvedTheme, DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from "../lib/themes/theme-registry";
import { applyTheme } from "../lib/themes/apply-theme";

const THEME_STORAGE_KEY = "deathpush:theme";
const PREFERRED_DARK_KEY = "deathpush:preferred-dark-theme";
const PREFERRED_LIGHT_KEY = "deathpush:preferred-light-theme";

const getPreferredDarkId = (): string => localStorage.getItem(PREFERRED_DARK_KEY) ?? DEFAULT_DARK_THEME_ID;
const getPreferredLightId = (): string => localStorage.getItem(PREFERRED_LIGHT_KEY) ?? DEFAULT_LIGHT_THEME_ID;

const resolveInitialTheme = (): ResolvedTheme => {
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  if (stored) {
    const theme = getResolvedTheme(stored);
    if (theme) return theme;
  }

  const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const defaultId = prefersDark ? getPreferredDarkId() : getPreferredLightId();
  return getResolvedTheme(defaultId) ?? getResolvedTheme(DEFAULT_DARK_THEME_ID)!;
};

interface ThemeState {
  currentTheme: ResolvedTheme;
  preferredDarkThemeId: string;
  preferredLightThemeId: string;
  setTheme: (id: string) => void;
  setPreferredDarkTheme: (id: string) => void;
  setPreferredLightTheme: (id: string) => void;
}

export const useThemeStore = create<ThemeState>((set) => ({
  currentTheme: resolveInitialTheme(),
  preferredDarkThemeId: getPreferredDarkId(),
  preferredLightThemeId: getPreferredLightId(),

  setTheme: (id: string) => {
    const theme = getResolvedTheme(id);
    if (!theme) return;
    applyTheme(theme);
    const updates: Partial<ThemeState> = { currentTheme: theme };
    if (theme.kind === "dark" || theme.kind === "hc-dark") {
      localStorage.setItem(PREFERRED_DARK_KEY, id);
      updates.preferredDarkThemeId = id;
    } else {
      localStorage.setItem(PREFERRED_LIGHT_KEY, id);
      updates.preferredLightThemeId = id;
    }
    set(updates);
  },

  setPreferredDarkTheme: (id: string) => {
    const theme = getResolvedTheme(id);
    if (!theme) return;
    localStorage.setItem(PREFERRED_DARK_KEY, id);
    set({ preferredDarkThemeId: id });
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (prefersDark) {
      applyTheme(theme);
      set({ currentTheme: theme });
    }
  },

  setPreferredLightTheme: (id: string) => {
    const theme = getResolvedTheme(id);
    if (!theme) return;
    localStorage.setItem(PREFERRED_LIGHT_KEY, id);
    set({ preferredLightThemeId: id });
    const prefersDark = window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (!prefersDark) {
      applyTheme(theme);
      set({ currentTheme: theme });
    }
  },
}));

const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
mediaQuery.addEventListener("change", (e) => {
  const state = useThemeStore.getState();
  const id = e.matches ? state.preferredDarkThemeId : state.preferredLightThemeId;
  const theme = getResolvedTheme(id);
  if (theme) {
    applyTheme(theme);
    useThemeStore.setState({ currentTheme: theme });
  }
});

window.addEventListener("storage", (e: StorageEvent) => {
  if (e.key === THEME_STORAGE_KEY && e.newValue) {
    const theme = getResolvedTheme(e.newValue);
    if (!theme) return;
    applyTheme(theme);
    useThemeStore.setState({ currentTheme: theme });
  }

  if (e.key === PREFERRED_DARK_KEY && e.newValue) {
    if (getResolvedTheme(e.newValue)) {
      useThemeStore.setState({ preferredDarkThemeId: e.newValue });
    }
  }

  if (e.key === PREFERRED_LIGHT_KEY && e.newValue) {
    if (getResolvedTheme(e.newValue)) {
      useThemeStore.setState({ preferredLightThemeId: e.newValue });
    }
  }
});
