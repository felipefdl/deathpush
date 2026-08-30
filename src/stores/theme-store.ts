import { createStore } from "zustand/vanilla";
import type { ResolvedTheme, ThemeKind } from "../lib/themes/theme-types";
import {
  DEFAULT_DARK_THEME_ID,
  DEFAULT_LIGHT_THEME_ID,
  THEME_ENTRIES,
  getDefaultResolvedTheme,
  getResolvedTheme,
} from "../lib/themes/theme-registry";
import { applyTheme } from "../lib/themes/apply-theme";

const THEME_STORAGE_KEY = "deathpush:theme";
const PREFERRED_DARK_KEY = "deathpush:preferred-dark-theme";
const PREFERRED_LIGHT_KEY = "deathpush:preferred-light-theme";
const themeKinds = new Map(THEME_ENTRIES.map((entry) => [entry.id, entry.kind]));

const storedThemeForKind = (key: string, kind: ThemeKind, fallback: string): string => {
  const stored = localStorage.getItem(key);
  return stored && themeKinds.get(stored) === kind ? stored : fallback;
};

const getPreferredDarkId = (): string => storedThemeForKind(PREFERRED_DARK_KEY, "dark", DEFAULT_DARK_THEME_ID);
const getPreferredLightId = (): string => storedThemeForKind(PREFERRED_LIGHT_KEY, "light", DEFAULT_LIGHT_THEME_ID);
const prefersDark = (): boolean => window.matchMedia("(prefers-color-scheme: dark)").matches;

interface ThemeState {
  currentTheme: ResolvedTheme;
  preferredDarkThemeId: string;
  preferredLightThemeId: string;
  setTheme: (id: string) => Promise<void>;
  setPreferredDarkTheme: (id: string) => Promise<void>;
  setPreferredLightTheme: (id: string) => Promise<void>;
}

let themeRequest = 0;

const activateTheme = async (id: string, persistPreference: boolean): Promise<void> => {
  const request = ++themeRequest;
  const theme = await getResolvedTheme(id);
  if (!theme || request !== themeRequest) return;

  const updates: Partial<ThemeState> = { currentTheme: theme };
  if (persistPreference) {
    const key = theme.kind === "dark" ? PREFERRED_DARK_KEY : PREFERRED_LIGHT_KEY;
    localStorage.setItem(key, id);
    if (theme.kind === "dark") updates.preferredDarkThemeId = id;
    else updates.preferredLightThemeId = id;
  }
  themeStore.setState(updates);
  applyTheme(theme);
};

export const themeStore = createStore<ThemeState>((set) => ({
  currentTheme: getDefaultResolvedTheme(prefersDark() ? "dark" : "light"),
  preferredDarkThemeId: getPreferredDarkId(),
  preferredLightThemeId: getPreferredLightId(),

  setTheme: (id) => activateTheme(id, true),

  setPreferredDarkTheme: async (id) => {
    if (themeKinds.get(id) !== "dark") return;
    localStorage.setItem(PREFERRED_DARK_KEY, id);
    set({ preferredDarkThemeId: id });
    if (prefersDark()) await activateTheme(id, false);
  },

  setPreferredLightTheme: async (id) => {
    if (themeKinds.get(id) !== "light") return;
    localStorage.setItem(PREFERRED_LIGHT_KEY, id);
    set({ preferredLightThemeId: id });
    if (!prefersDark()) await activateTheme(id, false);
  },
}));

export const initializeThemeStore = async (): Promise<void> => {
  const state = themeStore.getState();
  const stored = localStorage.getItem(THEME_STORAGE_KEY);
  const id =
    stored && themeKinds.has(stored)
      ? stored
      : prefersDark()
        ? state.preferredDarkThemeId
        : state.preferredLightThemeId;
  const theme = await getResolvedTheme(id);
  if (theme) themeStore.setState({ currentTheme: theme });
};

const mediaQuery = window.matchMedia("(prefers-color-scheme: dark)");
mediaQuery.addEventListener("change", (event) => {
  const state = themeStore.getState();
  const id = event.matches ? state.preferredDarkThemeId : state.preferredLightThemeId;
  if (id !== state.currentTheme.id) void activateTheme(id, false);
});

window.addEventListener("storage", (event: StorageEvent) => {
  if (event.key === THEME_STORAGE_KEY && event.newValue && themeKinds.has(event.newValue)) {
    void activateTheme(event.newValue, false);
    return;
  }
  if (event.key === PREFERRED_DARK_KEY && event.newValue && themeKinds.get(event.newValue) === "dark") {
    themeStore.setState({ preferredDarkThemeId: event.newValue });
  }
  if (event.key === PREFERRED_LIGHT_KEY && event.newValue && themeKinds.get(event.newValue) === "light") {
    themeStore.setState({ preferredLightThemeId: event.newValue });
  }
});
