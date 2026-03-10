export type Platform = "macos" | "windows" | "linux";

const ua = navigator.userAgent;
export const PLATFORM: Platform = ua.includes("Macintosh")
  ? "macos"
  : ua.includes("Windows")
    ? "windows"
    : "linux";

export const IS_MACOS = PLATFORM === "macos";
export const IS_WINDOWS = PLATFORM === "windows";
export const IS_LINUX = PLATFORM === "linux";
