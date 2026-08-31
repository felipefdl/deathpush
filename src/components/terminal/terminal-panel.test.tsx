import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { layoutStore } from "../../stores/layout-store";
import { repositoryStore } from "../../stores/repository-store";
import { shouldShowTerminalSidebar, TerminalPanel } from "./terminal-panel";

vi.mock("./terminal-group-view", () => ({
  TerminalGroupView: (props: { isActive: boolean; isFocused: boolean }) => (
    <div data-testid="terminal-group" data-active={String(props.isActive)} data-focused={String(props.isFocused)} />
  ),
}));

vi.mock("./git-output", () => ({
  GitOutput: () => <div data-testid="git-output" />,
}));

const onePane = {
  terminalGroups: [
    { groupId: 1, panes: [{ paneId: 1, name: "Terminal 1" }], activePaneId: 1, splitDirection: "horizontal" as const },
  ],
  activeGroupId: 1,
  terminalIdCounter: 1,
};

const twoPanes = {
  terminalGroups: [
    {
      groupId: 1,
      panes: [
        { paneId: 1, name: "Terminal 1" },
        { paneId: 2, name: "Terminal 2" },
      ],
      activePaneId: 1,
      splitDirection: "horizontal" as const,
    },
  ],
  activeGroupId: 1,
  terminalIdCounter: 2,
};

describe("shouldShowTerminalSidebar", () => {
  it("is hidden for a single pane and visible once there is more than one", () => {
    expect(shouldShowTerminalSidebar(1)).toBe(false);
    expect(shouldShowTerminalSidebar(2)).toBe(true);
  });
});

describe("TerminalPanel", () => {
  beforeEach(() => {
    layoutStore.setState({ panelTab: "terminal", terminalMaximized: false });
    repositoryStore.setState(onePane);
  });

  afterEach(() => {
    cleanup();
    layoutStore.setState({ panelTab: "terminal", terminalMaximized: false });
    repositoryStore.setState(onePane);
  });

  it("hides the terminal sidebar when only one pane exists", () => {
    const result = render(() => <TerminalPanel />);
    flush();

    expect(result.container.querySelector(".terminal-sidebar")).toBeNull();
  });

  it("hides the terminal sidebar when maximized with one pane", () => {
    layoutStore.setState({ terminalMaximized: true });
    const result = render(() => <TerminalPanel />);
    flush();

    expect(result.container.querySelector(".terminal-sidebar")).toBeNull();
  });

  it("shows the terminal sidebar when there is more than one pane", () => {
    repositoryStore.setState(twoPanes);
    const result = render(() => <TerminalPanel />);
    flush();

    expect(result.container.querySelector(".terminal-sidebar")).not.toBeNull();
  });

  it("keeps the terminal pane laid out while Output is active", () => {
    const result = render(() => <TerminalPanel />);
    flush();

    fireEvent.click(result.getByText("Output"));
    flush();

    const terminalMain = result.container.querySelector(".terminal-panel-main");
    expect(terminalMain).not.toBeNull();
    expect(terminalMain?.classList.contains("is-inactive")).toBe(true);
    expect((terminalMain as HTMLElement).style.display).not.toBe("none");
    expect(result.container.querySelector("[data-testid=terminal-group]")).not.toBeNull();
    expect(result.container.querySelector("[data-testid=terminal-group]")?.getAttribute("data-active")).toBe("true");
    expect(result.container.querySelector("[data-testid=terminal-group]")?.getAttribute("data-focused")).toBe("false");
  });
});
