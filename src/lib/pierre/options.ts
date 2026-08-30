import type { FileDiffOptions } from "@pierre/diffs";

export const PIERRE_SCROLLBAR_CSS = `
[data-code] {
  scrollbar-width: none;
}

[data-code]::-webkit-scrollbar {
  width: 0;
  height: 0;
}
`;

export const buildPierreDiffOptions = (args: {
  themeId: string;
  themeType: "light" | "dark";
  wordWrap: "off" | "on";
  diffMode: "inline" | "sideBySide";
  enableLineSelection: boolean;
}): FileDiffOptions<undefined> => ({
  theme: args.themeId,
  themeType: args.themeType,
  preferredHighlighter: "shiki-js" as const,
  disableFileHeader: true,
  diffStyle: args.diffMode === "sideBySide" ? "split" : "unified",
  overflow: args.wordWrap === "on" ? "wrap" : "scroll",
  unsafeCSS: PIERRE_SCROLLBAR_CSS,
  hunkSeparators: "line-info-basic" as const,
  enableLineSelection: args.enableLineSelection,
  lineDiffType: "word-alt" as const,
  disableBackground: false,
  diffIndicators: "none" as const,
});
