export type ThemeKind = "dark" | "light" | "hc-dark" | "hc-light";

export type UiTheme = "vs" | "vs-dark" | "hc-black" | "hc-light";

export interface VscodeThemeJson {
  $schema?: string;
  name?: string;
  include?: string;
  type?: string;
  colors?: Record<string, string>;
  tokenColors?: TokenColor[];
  semanticTokenColors?: Record<string, string | TokenColorSettings>;
}

export interface TokenColor {
  name?: string;
  scope?: string | string[];
  settings: TokenColorSettings;
}

export interface TokenColorSettings {
  foreground?: string;
  background?: string;
  fontStyle?: string;
}

export interface ThemeEntry {
  id: string;
  label: string;
  description?: string;
  uiTheme: UiTheme;
  kind: ThemeKind;
}

export interface ResolvedTheme extends ThemeEntry {
  colors: Record<string, string>;
  tokenColors: TokenColor[];
}
