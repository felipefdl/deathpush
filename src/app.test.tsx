import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import type { RepositoryStatus } from "./lib/git-types";
import { explorerStore } from "./stores/explorer-store";
import { layoutStore } from "./stores/layout-store";
import { repositoryStore } from "./stores/repository-store";
import { App } from "./app";

const { getInitialPathMock, listenMock, openRepoMock, setRepoMenuEnabledMock, setZoomMock } = vi.hoisted(() => ({
  getInitialPathMock: vi.fn(async (): Promise<string | null> => null),
  listenMock: vi.fn(async () => vi.fn()),
  openRepoMock: vi.fn(async () => {}),
  setRepoMenuEnabledMock: vi.fn(async () => {}),
  setZoomMock: vi.fn(async () => {}),
}));

vi.mock("@tauri-apps/api/webviewWindow", () => ({
  getCurrentWebviewWindow: () => ({
    label: "main",
    listen: listenMock,
    setZoom: setZoomMock,
  }),
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({
  confirm: vi.fn(async () => false),
  message: vi.fn(async () => {}),
  open: vi.fn(async () => null),
}));

vi.mock("./components/layout/app-layout", () => ({ AppLayout: () => <div data-testid="app-layout" /> }));
vi.mock("./components/layout/linux-title-bar", () => ({ LinuxTitleBar: () => null }));
vi.mock("./components/welcome/welcome-screen", () => ({
  WelcomeScreen: () => <div data-testid="welcome-screen" />,
}));
vi.mock("./hooks/use-keyboard-shortcuts", () => ({ useKeyboardShortcuts: vi.fn() }));
vi.mock("./hooks/use-repository", () => ({ useRepository: () => ({ openRepo: openRepoMock }) }));
vi.mock("./hooks/use-stash", () => ({ useStash: () => ({ popStash: vi.fn(), saveStash: vi.fn() }) }));
vi.mock("./lib/tauri-commands", () => ({
  getInitialPath: getInitialPathMock,
  setRepoMenuEnabled: setRepoMenuEnabledMock,
}));

const STATUS: RepositoryStatus = {
  root: "/test/project",
  headBranch: "main",
  headCommit: "abc123",
  ahead: 0,
  behind: 0,
  groups: [],
  operationState: "none",
};

describe("App project refresh", () => {
  beforeEach(() => {
    vi.stubGlobal("matchMedia", () => ({
      matches: true,
      addEventListener: vi.fn(),
      removeEventListener: vi.fn(),
    }));
    localStorage.clear();
    getInitialPathMock.mockReset();
    getInitialPathMock.mockResolvedValue(null);
    openRepoMock.mockReset();
    openRepoMock.mockResolvedValue();
    repositoryStore.setState({ status: null, error: null });
    explorerStore.getState().reset();
    layoutStore.setState({ mainView: "changes", sidebarView: "scm" });
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
  });

  it("preserves the selected Explorer file when the same repository refreshes", () => {
    const loadForProject = vi.spyOn(layoutStore.getState(), "loadForProject");
    const resetExplorer = vi.spyOn(explorerStore.getState(), "reset");
    render(() => <App />);
    flush();

    repositoryStore.getState().setStatus(STATUS);
    flush();
    expect(loadForProject).toHaveBeenCalledTimes(1);
    expect(resetExplorer).toHaveBeenCalledTimes(1);

    layoutStore.getState().setMainView("file");
    explorerStore.getState().setSelectedPath("src/app.tsx");

    repositoryStore.getState().setStatus({ ...STATUS });
    flush();

    expect(loadForProject).toHaveBeenCalledTimes(1);
    expect(resetExplorer).toHaveBeenCalledTimes(1);
    expect(layoutStore.getState().mainView).toBe("file");
    expect(explorerStore.getState().selectedPath).toBe("src/app.tsx");
  });

  it("shows the skull while the app is starting", () => {
    getInitialPathMock.mockReturnValue(new Promise<string | null>(() => {}));
    const result = render(() => <App />);
    flush();

    expect(result.container.querySelector(".boot-splash")).toBeTruthy();
  });

  it("paints cached repository identity before the initial path resolves", async () => {
    let resolveInitialPath!: (path: string | null) => void;
    getInitialPathMock.mockReturnValue(
      new Promise<string | null>((resolve) => {
        resolveInitialPath = resolve;
      })
    );
    openRepoMock.mockReturnValue(new Promise<void>(() => {}));
    localStorage.setItem(
      "deathpush:recentProjects",
      JSON.stringify([
        {
          path: "/test/cached-project",
          name: "cached-project",
          branch: "feat/cached",
          lastOpened: "2026-08-31T12:00:00.000Z",
        },
      ])
    );

    const result = render(() => <App />);
    flush();

    const cachedChrome = result.container.querySelector(".cached-repository-chrome");
    expect(cachedChrome?.textContent).toContain("cached-project");
    expect(cachedChrome?.textContent).toContain("feat/cached");
    expect(openRepoMock).not.toHaveBeenCalled();

    resolveInitialPath(null);
    await Promise.resolve();
    await Promise.resolve();
    flush();

    expect(openRepoMock).toHaveBeenCalledWith("/test/cached-project");
    expect(result.container.querySelector(".cached-repository-chrome")).toBeTruthy();
  });

  it("shows welcome after startup when no cached identity exists", async () => {
    const result = render(() => <App />);
    flush();
    await Promise.resolve();
    await Promise.resolve();
    flush();

    expect(result.getByTestId("welcome-screen")).toBeTruthy();
  });

  it("lets a CLI path supersede cached identity", async () => {
    localStorage.setItem(
      "deathpush:recentProjects",
      JSON.stringify([
        {
          path: "/test/cached-project",
          name: "cached-project",
          branch: "feat/cached",
          lastOpened: "2026-08-31T12:00:00.000Z",
        },
      ])
    );
    getInitialPathMock.mockResolvedValue("/test/cli-project");
    openRepoMock.mockReturnValue(new Promise<void>(() => {}));

    const result = render(() => <App />);
    flush();
    await Promise.resolve();
    await Promise.resolve();
    flush();

    expect(openRepoMock).toHaveBeenCalledOnce();
    expect(openRepoMock).toHaveBeenCalledWith("/test/cli-project");
    expect(result.container.querySelector(".cached-repository-chrome")).toBeNull();
  });

  it("leaves the splash without waiting for a startup repo to finish opening", async () => {
    getInitialPathMock.mockResolvedValue("/test/project");
    openRepoMock.mockReturnValue(new Promise<void>(() => {}));
    const result = render(() => <App />);
    flush();
    await Promise.resolve();
    await Promise.resolve();
    flush();

    expect(result.container.querySelector(".boot-splash")).toBeNull();
  });
});
