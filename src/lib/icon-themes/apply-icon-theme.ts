import type { ResolvedIconTheme } from "./icon-theme-types";

const STYLE_ID = "deathpush-icon-theme-css";
const STORAGE_KEY = "deathpush:iconTheme";

export const applyIconTheme = (theme: ResolvedIconTheme): void => {
  let style = document.getElementById(STYLE_ID) as HTMLStyleElement | null;

  if (theme.id === "none") {
    if (style) style.textContent = "";
    document.body.classList.remove("show-file-icons");
    localStorage.setItem(STORAGE_KEY, theme.id);
    return;
  }

  if (!style) {
    style = document.createElement("style");
    style.id = STYLE_ID;
    document.head.appendChild(style);
  }

  style.textContent = theme.cssContent;
  document.body.classList.add("show-file-icons");
  localStorage.setItem(STORAGE_KEY, theme.id);
};
