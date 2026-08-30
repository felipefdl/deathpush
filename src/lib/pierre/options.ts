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
  showLineNumbers: boolean;
  diffIndicators: "classic" | "bars" | "none";
  lineDiffType: "word-alt" | "word" | "char" | "none";
  showBackground: boolean;
  hunkSeparators: "simple" | "metadata" | "line-info" | "line-info-basic";
}): FileDiffOptions<undefined> => ({
  theme: args.themeId,
  themeType: args.themeType,
  preferredHighlighter: "shiki-js" as const,
  disableFileHeader: true,
  disableLineNumbers: !args.showLineNumbers,
  diffStyle: args.diffMode === "sideBySide" ? "split" : "unified",
  overflow: args.wordWrap === "on" ? "wrap" : "scroll",
  unsafeCSS: PIERRE_SCROLLBAR_CSS,
  hunkSeparators: args.hunkSeparators,
  enableLineSelection: args.enableLineSelection,
  lineDiffType: args.lineDiffType,
  disableBackground: !args.showBackground,
  diffIndicators: args.diffIndicators,
});
