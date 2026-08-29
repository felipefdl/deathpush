import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { repositoryStore } from "../../stores/repository-store";
import { WelcomeScreen } from "./welcome-screen";

vi.mock("../../lib/tauri-commands", () => ({
  scanProjectsDirectory: vi.fn(async () => []),
}));

vi.mock("../../lib/updater", () => ({
  checkForUpdate: vi.fn(async () => null),
  downloadAndInstallUpdate: vi.fn(),
}));

describe("WelcomeScreen opening state", () => {
  afterEach(() => {
    cleanup();
    repositoryStore.getState().endOperation("open-repo");
  });

  it("shows opening feedback while a repository is loading", () => {
    repositoryStore.getState().startOperation("open-repo");
    const result = render(() => (
      <WelcomeScreen onOpenRepository={vi.fn()} onCloneRepository={vi.fn()} onSelectProject={vi.fn()} />
    ));
    flush();

    expect(result.getByText("Opening repository...")).toBeTruthy();
  });

  it("hides opening feedback when idle", () => {
    const result = render(() => (
      <WelcomeScreen onOpenRepository={vi.fn()} onCloneRepository={vi.fn()} onSelectProject={vi.fn()} />
    ));
    flush();

    expect(result.queryByText("Opening repository...")).toBeNull();
  });
});
