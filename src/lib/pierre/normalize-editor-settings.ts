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
  overflow: "auto";
  "font-family": string;
  "font-size": string;
  "line-height": string;
  "tab-size": number;
} => ({
  width: "100%",
  height: "100%",
  overflow: "auto",
  "font-family": settings.fontFamily,
  "font-size": `${settings.fontSize}px`,
  "line-height": `${settings.lineHeight}px`,
  "tab-size": settings.tabSize,
});
