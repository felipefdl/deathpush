import { describe, expect, it, vi } from "vite-plus/test";
import { confirmWindowClose } from "./window-close";

const { confirmMock, terminalsHaveActiveProcessMock } = vi.hoisted(() => ({
  confirmMock: vi.fn(async () => false),
  terminalsHaveActiveProcessMock: vi.fn(async () => false),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: confirmMock,
}));

vi.mock("./tauri-commands", () => ({
  terminalsHaveActiveProcess: terminalsHaveActiveProcessMock,
}));

describe("confirmWindowClose", () => {
  it("closes immediately when no terminal process is running", async () => {
    terminalsHaveActiveProcessMock.mockResolvedValue(false);
    await expect(confirmWindowClose()).resolves.toBe(true);
    expect(confirmMock).not.toHaveBeenCalled();
  });

  it("asks before closing when a terminal process is running", async () => {
    terminalsHaveActiveProcessMock.mockResolvedValue(true);
    confirmMock.mockResolvedValue(true);
    await expect(confirmWindowClose()).resolves.toBe(true);
    expect(confirmMock).toHaveBeenCalled();
  });

  it("cancels close when the running-process confirm is dismissed", async () => {
    terminalsHaveActiveProcessMock.mockResolvedValue(true);
    confirmMock.mockResolvedValue(false);
    await expect(confirmWindowClose()).resolves.toBe(false);
  });
});
