import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { settingsStore } from "../../stores/settings-store";
import { SettingsPage } from "./settings-page";

const { confirmMock } = vi.hoisted(() => ({
  confirmMock: vi.fn(async () => false),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: confirmMock,
}));

vi.mock("../../lib/tauri-commands", () => ({
  getGitConfig: vi.fn(async () => ""),
  setGitConfig: vi.fn(async () => {}),
}));

describe("SettingsPage controls", () => {
  afterEach(() => {
    cleanup();
    confirmMock.mockReset();
    confirmMock.mockResolvedValue(false);
  });

  it("uses a step that accepts the default color saturation", () => {
    const result = render(() => <SettingsPage />);
    const label = [...result.container.querySelectorAll(".settings-label")].find(
      (element) => element.textContent === "Color Saturation"
    )!;
    const input = label.parentElement!.querySelector<HTMLInputElement>('input[type="number"]')!;

    expect(input.step).toBe("0.01");
    expect(input.checkValidity()).toBe(true);
  });

  it("gives every Settings control an accessible name", () => {
    const result = render(() => <SettingsPage />);

    expect(result.getByRole("textbox", { name: "UI Font Family" })).toBeTruthy();
    expect(result.getByRole("switch", { name: "Git Blame" })).toBeTruthy();
    expect(result.getByRole("textbox", { name: "Workspace Directories" })).toBeTruthy();
    expect(result.getByRole("combobox", { name: "Shell Path" })).toBeTruthy();
    expect(result.getByRole("combobox", { name: "Bell Style" })).toBeTruthy();
    expect(result.getByRole("spinbutton", { name: "Color Saturation" })).toBeTruthy();
  });

  it("offers focused Pierre diff customization controls", () => {
    const result = render(() => <SettingsPage />);

    expect(result.getByRole("combobox", { name: "Diff Layout" })).toBeTruthy();
    expect(result.getByRole("switch", { name: "Inline Hunk Actions" })).toBeTruthy();
    expect(result.getByRole("switch", { name: "Line Numbers" })).toBeTruthy();
    expect(result.getByRole("switch", { name: "Background Highlighting" })).toBeTruthy();
    expect(result.getByRole("combobox", { name: "Diff Indicators" })).toBeTruthy();
    expect(result.getByRole("combobox", { name: "Inline Changes" })).toBeTruthy();
    expect(result.getByRole("combobox", { name: "Hunk Separators" })).toBeTruthy();
  });

  it("shows only supported terminal behavior controls", () => {
    const result = render(() => <SettingsPage />);

    expect(result.getByRole("switch", { name: "Right Click Selects Word" })).toBeTruthy();
    expect(result.getByRole("switch", { name: "macOS Option Click Forces Selection" })).toBeTruthy();
    expect(result.queryByRole("spinbutton", { name: "Scroll Sensitivity" })).toBeNull();
    expect(result.queryByRole("spinbutton", { name: "Fast Scroll Sensitivity" })).toBeNull();
    expect(result.queryByRole("spinbutton", { name: "Smooth Scroll Duration" })).toBeNull();
    expect(result.queryByRole("switch", { name: "Scroll on User Input" })).toBeNull();
    expect(result.queryByRole("switch", { name: "Alt Click Moves Cursor" })).toBeNull();
    expect(result.queryByRole("switch", { name: "macOS Option as Meta" })).toBeNull();
    expect(result.queryByRole("switch", { name: "Draw Bold Text in Bright Colors" })).toBeNull();
    expect(result.queryByRole("spinbutton", { name: "Minimum Contrast Ratio" })).toBeNull();
    expect(result.queryByRole("switch", { name: "Rescale Overlapping Glyphs" })).toBeNull();
    expect(result.queryByRole("spinbutton", { name: "Tab Stop Width" })).toBeNull();
    expect(result.queryByRole("textbox", { name: "Word Separator" })).toBeNull();
  });

  it("limits word wrap to Off and On and drops render whitespace", () => {
    const result = render(() => <SettingsPage />);
    const wrap = result.getByRole("combobox", { name: "Word Wrap" });
    expect([...wrap.querySelectorAll("option")].map((option) => [option.value, option.textContent])).toEqual([
      ["off", "Off"],
      ["on", "On"],
    ]);
    expect(result.queryByRole("combobox", { name: "Render Whitespace" })).toBeNull();
  });

  it("offers the built-in Trees density and icon presets", () => {
    const result = render(() => <SettingsPage />);
    const density = result.getByRole("combobox", { name: "Tree Density" });
    const icons = result.getByRole("combobox", { name: "Tree Icons" });

    expect([...density.querySelectorAll("option")].map((option) => [option.value, option.textContent])).toEqual([
      ["compact", "Compact"],
      ["default", "Default"],
      ["relaxed", "Relaxed"],
    ]);
    expect([...icons.querySelectorAll("option")].map((option) => [option.value, option.textContent])).toEqual([
      ["minimal", "Minimal"],
      ["standard", "Standard"],
      ["complete", "Complete"],
    ]);
    expect(result.queryByText("File Icon Theme")).toBeNull();
  });

  it("does not reset settings when the confirm dialog is cancelled", async () => {
    confirmMock.mockResolvedValue(false);
    const resetToDefaults = vi.spyOn(settingsStore.getState(), "resetToDefaults").mockImplementation(() => {});
    const result = render(() => <SettingsPage />);

    fireEvent.click(result.getByRole("button", { name: "Reset to Defaults" }));
    await Promise.resolve();
    flush();

    expect(confirmMock).toHaveBeenCalled();
    expect(resetToDefaults).not.toHaveBeenCalled();
  });

  it("resets settings after the confirm dialog is accepted", async () => {
    confirmMock.mockResolvedValue(true);
    const resetToDefaults = vi.spyOn(settingsStore.getState(), "resetToDefaults").mockImplementation(() => {});
    const result = render(() => <SettingsPage />);

    fireEvent.click(result.getByRole("button", { name: "Reset to Defaults" }));
    await Promise.resolve();
    flush();

    expect(confirmMock).toHaveBeenCalled();
    expect(resetToDefaults).toHaveBeenCalledTimes(1);
  });
});
