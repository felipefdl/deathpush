import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app";
import { useThemeStore } from "./stores/theme-store";
import { applyTheme } from "./lib/themes/apply-theme";
import { useIconThemeStore } from "./stores/icon-theme-store";
import { applyIconTheme } from "./lib/icon-themes/apply-icon-theme";
import { useSettingsStore } from "./stores/settings-store";
import "./styles/global.css";

document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement;
  if (target.closest(".xterm") || target.closest(".monaco-editor")) {
    return;
  }
  e.preventDefault();
});

applyTheme(useThemeStore.getState().currentTheme);
applyIconTheme(useIconThemeStore.getState().currentIconTheme);

const uiSettings = useSettingsStore.getState().settings.ui;
document.documentElement.style.setProperty("--vscode-font-family", uiSettings.fontFamily);
document.documentElement.style.setProperty("--vscode-font-size", `${uiSettings.fontSize}px`);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
