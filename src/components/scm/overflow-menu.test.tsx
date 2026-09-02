import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { OverflowMenu } from "./overflow-menu";

const { repositoryState } = vi.hoisted(() => ({
  repositoryState: {
    status: null,
    stashes: [],
    branches: [],
    operations: new Set<string>(),
    actions: null,
    setIdentity: vi.fn(),
    setError: vi.fn(),
    startOperation: vi.fn(),
    endOperation: vi.fn(),
  },
}));


vi.mock("../../stores/repository-store", () => ({
  repositoryStore: { getState: () => repositoryState },
}));

vi.mock("../../lib/use-store", () => ({
  useStore: (_store: unknown, selector: (state: typeof repositoryState) => unknown) => () => selector(repositoryState),
}));

vi.mock("../../hooks/use-stash", () => ({
  useStash: () => ({
    saveStash: vi.fn(),
    saveStashIncludeUntracked: vi.fn(),
    saveStashStaged: vi.fn(),
    popStash: vi.fn(),
  }),
}));

vi.mock("../../hooks/use-branches", () => ({
  useBranches: () => ({ mergeBranch: vi.fn(), rebaseBranch: vi.fn() }),
}));


describe("OverflowMenu", () => {
  afterEach(() => cleanup());

  it("keeps the menu inside the window when the sidebar is narrower than the menu", () => {
    const anchor = document.createElement("button");
    anchor.getBoundingClientRect = () =>
      ({
        left: 168,
        right: 190,
        top: 20,
        bottom: 42,
        width: 22,
        height: 22,
        x: 168,
        y: 20,
        toJSON: () => ({}),
      }) as DOMRect;
    Object.defineProperty(window, "innerWidth", { configurable: true, value: 228 });

    render(() => (
      <OverflowMenu anchorRef={anchor} onClose={() => {}} onOpenRepository={() => {}} onCloneRepository={() => {}} />
    ));
    flush();

    const menu = document.body.querySelector<HTMLElement>(".overflow-menu");
    expect(menu?.style.position).toBe("fixed");
    expect(menu?.style.left).toBe("8px");
    expect(menu?.style.top).toBe("42px");
  });
});
