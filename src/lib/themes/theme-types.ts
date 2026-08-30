export type ThemeKind = "dark" | "light";

export type TokenColor = {
  name?: string;
  scope?: string | string[];
  settings: {
    foreground?: string;
    background?: string;
    fontStyle?: string;
  };
};

export type ThemeEntry = {
  id: string;
  label: string;
  kind: ThemeKind;
};

export type ResolvedTheme = {
  id: string;
  label: string;
  kind: ThemeKind;
  name: string;
  type: ThemeKind;
  bg: string;
  fg: string;
  colors: Record<string, string>;
  tokenColors: TokenColor[];
};
