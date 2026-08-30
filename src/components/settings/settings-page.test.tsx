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

  it("limits word wrap to Off and On and drops render whitespace", () => {
    const result = render(() => <SettingsPage />);
    const wrap = result.getByRole("combobox", { name: "Word Wrap" });
    expect([...wrap.querySelectorAll("option")].map((option) => [option.value, option.textContent])).toEqual([
      ["off", "Off"],
      ["on", "On"],
    ]);
    expect(result.queryByRole("combobox", { name: "Render Whitespace" })).toBeNull();
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
