import { render } from "@solidjs/web";
import { App } from "./app";
import { initializeThemeStore, themeStore } from "./stores/theme-store";
import { applyTheme } from "./lib/themes/apply-theme";
import { getBootTheme } from "./lib/themes/boot-theme";
import { settingsStore } from "./stores/settings-store";
import "./styles/global.css";

document.addEventListener("contextmenu", (e) => {
  const target = e.target as HTMLElement;
  const tag = target.tagName;
  if (tag === "INPUT" || tag === "TEXTAREA" || target.isContentEditable) {
    return;
  }
  if (target.closest(".wterm") || target.closest(".pierre-file-host")) {
    return;
  }
  e.preventDefault();
});

applyTheme(getBootTheme(), { transient: true });
void initializeThemeStore()
  .then(() => {
    applyTheme(themeStore.getState().currentTheme);
  })
  .catch(() => {});

const uiSettings = settingsStore.getState().settings.ui;
document.documentElement.style.setProperty("--vscode-font-family", uiSettings.fontFamily);
document.documentElement.style.setProperty("--vscode-font-size", `${uiSettings.fontSize}px`);

render(() => <App />, document.getElementById("root")!);

await document.fonts.ready;
await new Promise<void>((resolve) => {
  requestAnimationFrame(() => resolve());
});
document.getElementById("boot-splash")?.remove();
