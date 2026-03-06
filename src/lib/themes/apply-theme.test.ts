import { describe, it, expect, vi } from "vitest";

vi.mock("@monaco-editor/react", () => ({ loader: { init: vi.fn() } }));

import { getTerminalTheme } from "./apply-theme";

describe("getTerminalTheme", () => {
  it("uses provided values when all colors are given", () => {
    const colors: Record<string, string> = {
      "terminal.background": "#000001",
      "terminal.foreground": "#000002",
      "editor.background": "#FFFFFF",
      "editor.foreground": "#FFFFFF",
      "terminalCursor.foreground": "#000003",
      "terminalCursor.background": "#000004",
      "terminal.selectionBackground": "#000005",
      "terminal.ansiBlack": "#100000",
      "terminal.ansiRed": "#200000",
      "terminal.ansiGreen": "#300000",
      "terminal.ansiYellow": "#400000",
      "terminal.ansiBlue": "#500000",
      "terminal.ansiMagenta": "#600000",
      "terminal.ansiCyan": "#700000",
      "terminal.ansiWhite": "#800000",
      "terminal.ansiBrightBlack": "#900000",
      "terminal.ansiBrightRed": "#A00000",
      "terminal.ansiBrightGreen": "#B00000",
      "terminal.ansiBrightYellow": "#C00000",
      "terminal.ansiBrightBlue": "#D00000",
      "terminal.ansiBrightMagenta": "#E00000",
      "terminal.ansiBrightCyan": "#F00000",
      "terminal.ansiBrightWhite": "#FF0000",
    };
    const result = getTerminalTheme(colors);
    expect(result.background).toBe("#000001");
    expect(result.foreground).toBe("#000002");
    expect(result.cursor).toBe("#000003");
    expect(result.cursorAccent).toBe("#000004");
    expect(result.selectionBackground).toBe("#000005");
    expect(result.black).toBe("#100000");
    expect(result.red).toBe("#200000");
    expect(result.green).toBe("#300000");
    expect(result.brightBlue).toBe("#D00000");
    expect(result.brightMagenta).toBe("#E00000");
    expect(result.brightWhite).toBe("#FF0000");
  });

  it("uses all fallback defaults for an empty colors object", () => {
    const result = getTerminalTheme({});
    expect(result.background).toBe("#1E1E1E");
    expect(result.foreground).toBe("#CCCCCC");
    expect(result.cursor).toBe("#AEAFAD");
    expect(result.cursorAccent).toBe("#000000");
    expect(result.selectionBackground).toBe("rgba(255, 255, 255, 0.3)");
    expect(result.black).toBe("#000000");
    expect(result.red).toBe("#CD3131");
    expect(result.green).toBe("#0DBC79");
    expect(result.yellow).toBe("#E5E510");
    expect(result.blue).toBe("#2472C8");
    expect(result.magenta).toBe("#BC3FBC");
    expect(result.cyan).toBe("#11A8CD");
    expect(result.white).toBe("#E5E5E5");
    expect(result.brightBlack).toBe("#666666");
    expect(result.brightRed).toBe("#F14C4C");
    expect(result.brightGreen).toBe("#23D18B");
    expect(result.brightYellow).toBe("#F5F543");
    expect(result.brightBlue).toBe("#3B8EEA");
    expect(result.brightMagenta).toBe("#D670D6");
    expect(result.brightCyan).toBe("#29B8DB");
    expect(result.brightWhite).toBe("#E5E5E5");
  });

  it("falls back to editor.background when terminal.background is missing", () => {
    const result = getTerminalTheme({ "editor.background": "#AABBCC" });
    expect(result.background).toBe("#AABBCC");
  });

  it("falls back to hardcoded default when both terminal and editor background are missing", () => {
    const result = getTerminalTheme({});
    expect(result.background).toBe("#1E1E1E");
  });

  it("falls back to editor.foreground when terminal.foreground is missing", () => {
    const result = getTerminalTheme({ "editor.foreground": "#DDEEFF" });
    expect(result.foreground).toBe("#DDEEFF");
  });

  it("falls back to hardcoded default when both terminal and editor foreground are missing", () => {
    const result = getTerminalTheme({});
    expect(result.foreground).toBe("#CCCCCC");
  });

  it("falls back to default cursor color when terminalCursor.foreground is missing", () => {
    const result = getTerminalTheme({});
    expect(result.cursor).toBe("#AEAFAD");
  });

  it("falls back correctly for individual ANSI colors - black", () => {
    const result = getTerminalTheme({});
    expect(result.black).toBe("#000000");
  });

  it("falls back correctly for individual ANSI colors - red", () => {
    const result = getTerminalTheme({});
    expect(result.red).toBe("#CD3131");
  });

  it("falls back correctly for individual ANSI colors - green", () => {
    const result = getTerminalTheme({});
    expect(result.green).toBe("#0DBC79");
  });

  it("falls back correctly for individual ANSI colors - brightBlue", () => {
    const result = getTerminalTheme({});
    expect(result.brightBlue).toBe("#3B8EEA");
  });

  it("falls back correctly for individual ANSI colors - brightMagenta", () => {
    const result = getTerminalTheme({});
    expect(result.brightMagenta).toBe("#D670D6");
  });

  it("falls back correctly for individual ANSI colors - brightWhite", () => {
    const result = getTerminalTheme({});
    expect(result.brightWhite).toBe("#E5E5E5");
  });

  it("uses provided value for some ANSI colors and falls back for others", () => {
    const result = getTerminalTheme({
      "terminal.ansiRed": "#FF0000",
      "terminal.ansiBrightCyan": "#00FFFF",
    });
    expect(result.red).toBe("#FF0000");
    expect(result.brightCyan).toBe("#00FFFF");
    expect(result.black).toBe("#000000");
    expect(result.brightWhite).toBe("#E5E5E5");
  });
});
