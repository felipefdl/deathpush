import type { FileDiffOptions } from "@pierre/diffs";

export const buildPierreDiffOptions = (args: {
  themeId: string;
  wordWrap: "off" | "on";
  diffMode: "inline" | "sideBySide";
  enableLineSelection: boolean;
}): FileDiffOptions<undefined> => ({
  theme: args.themeId,
  themeType: undefined,
  preferredHighlighter: "shiki-js" as const,
  disableFileHeader: true,
  diffStyle: args.diffMode === "sideBySide" ? "split" : "unified",
  overflow: args.wordWrap === "on" ? "wrap" : "scroll",
  hunkSeparators: "line-info-basic" as const,
  enableLineSelection: args.enableLineSelection,
  lineDiffType: "word-alt" as const,
  disableBackground: false,
  diffIndicators: "none" as const,
});
