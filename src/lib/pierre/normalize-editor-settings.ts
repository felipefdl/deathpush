export const normalizeWordWrap = (value: string | undefined): "off" | "on" => {
  if (value === "off") return "off";
  return "on";
};
