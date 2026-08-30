import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import type { ResolvedTheme } from "../../lib/themes/theme-types";
import { ThemePicker } from "./theme-picker";

const { applyThemeMock, events, setStateMock, setThemeMock, themes } = vi.hoisted(() => {
  const eventLog: string[] = [];
  const resolvedThemes: ResolvedTheme[] = [
    {
      id: "test-theme",
      label: "Test Theme",
      kind: "dark",
      name: "test-theme",
      type: "dark",
      bg: "#000000",
      fg: "#ffffff",
      colors: {},
      tokenColors: [],
    },
    {
      id: "other-theme",
      label: "Other Theme",
      kind: "dark",
      name: "other-theme",
      type: "dark",
      bg: "#000000",
      fg: "#ffffff",
      colors: {},
      tokenColors: [],
    },
  ];
  return {
    applyThemeMock: vi.fn(),
    events: eventLog,
    setStateMock: vi.fn(),
    setThemeMock: vi.fn(async () => {
      eventLog.push("setTheme");
    }),
    themes: resolvedThemes,
  };
});

vi.mock("../../stores/theme-store", () => ({
  themeStore: {
    getState: () => ({ currentTheme: themes[0], setTheme: setThemeMock }),
    setState: setStateMock,
  },
}));

vi.mock("../../lib/themes/apply-theme", () => ({
  applyTheme: applyThemeMock,
}));

vi.mock("../../lib/themes/theme-registry", () => ({
  THEME_ENTRIES: themes,
  getResolvedTheme: async (id: string) => themes.find((theme) => theme.id === id),
}));

describe("ThemePicker", () => {
  beforeEach(() => {
    events.length = 0;
    setThemeMock.mockClear();
    setStateMock.mockClear();
    applyThemeMock.mockClear();
    vi.stubGlobal("matchMedia", () => ({ matches: true }));
    vi.stubGlobal("scrollTo", vi.fn());
    Element.prototype.scrollIntoView = vi.fn();
  });

  afterEach(() => {
    cleanup();
    vi.unstubAllGlobals();
  });

  it("does not apply themes while the mouse moves through the list", () => {
    const result = render(() => <ThemePicker onClose={vi.fn()} />);
    flush();
    applyThemeMock.mockClear();

    const items = result.container.querySelectorAll("[data-theme-item]");
    fireEvent.mouseMove(result.container.querySelector(".theme-picker-list")!);
    fireEvent.mouseEnter(items[0]);
    flush();

    expect(applyThemeMock).not.toHaveBeenCalled();
  });

  it("closes before applying the selected theme", () => {
    const result = render(() => <ThemePicker onClose={() => events.push("close")} />);
    flush();

    fireEvent.click(result.container.querySelector("[data-theme-item]")!);
    flush();

    expect(events).toEqual(["close", "setTheme"]);
  });

  it("updates theme state during keyboard previews", async () => {
    const result = render(() => <ThemePicker onClose={vi.fn()} />);
    flush();

    fireEvent.keyDown(result.container.querySelector("input")!, { key: "ArrowDown" });

    await vi.waitFor(() => {
      expect(setStateMock).toHaveBeenCalledWith({ currentTheme: themes[1] });
    });
    expect(applyThemeMock).toHaveBeenCalledWith(themes[1], { transient: true });
  });
});
