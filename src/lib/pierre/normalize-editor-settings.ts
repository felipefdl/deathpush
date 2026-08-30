export const normalizeWordWrap = (value: string | undefined): "off" | "on" => {
  if (value === "off") return "off";
  return "on";
};

export type PierreHostMetrics = {
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  tabSize: number;
};

export const pierreHostStyle = (
  settings: PierreHostMetrics
): {
  width: "100%";
  height: "100%";
  "min-width": "0";
  "overflow-x": "hidden";
  "overflow-y": "auto";
  display: "flex";
  "flex-direction": "column";
  "background-color": "var(--vscode-editor-background)";
  "--diffs-gap-block": "0px";
  "--diffs-font-family": string;
  "--diffs-font-size": string;
  "--diffs-line-height": string;
  "--diffs-tab-size": number;
  "--diffs-light-bg": "var(--vscode-editor-background)";
  "--diffs-dark-bg": "var(--vscode-editor-background)";
  "--diffs-light": "var(--vscode-editor-foreground)";
  "--diffs-dark": "var(--vscode-editor-foreground)";
  "font-family": string;
  "font-size": string;
  "line-height": string;
  "tab-size": number;
} => ({
  width: "100%",
  height: "100%",
  "min-width": "0",
  "overflow-x": "hidden",
  "overflow-y": "auto",
  display: "flex",
  "flex-direction": "column",
  "background-color": "var(--vscode-editor-background)",
  "--diffs-gap-block": "0px",
  "--diffs-font-family": settings.fontFamily,
  "--diffs-font-size": `${settings.fontSize}px`,
  "--diffs-line-height": `${settings.lineHeight}px`,
  "--diffs-tab-size": settings.tabSize,
  "--diffs-light-bg": "var(--vscode-editor-background)",
  "--diffs-dark-bg": "var(--vscode-editor-background)",
  "--diffs-light": "var(--vscode-editor-foreground)",
  "--diffs-dark": "var(--vscode-editor-foreground)",
  "font-family": settings.fontFamily,
  "font-size": `${settings.fontSize}px`,
  "line-height": `${settings.lineHeight}px`,
  "tab-size": settings.tabSize,
});
