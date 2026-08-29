import { beforeEach, describe, expect, it } from "vite-plus/test";
import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";
import { toggleTerminal } from "./toggle-terminal";

beforeEach(() => {
  layoutStore.setState({
    terminalVisible: false,
    terminalMaximized: false,
  });
  repositoryStore.setState({
    terminalGroups: [{ groupId: 1, panes: [{ paneId: 1, name: "Terminal 1" }], activePaneId: 1, splitDirection: "horizontal" }],
    activeGroupId: 1,
    terminalIdCounter: 1,
  });
});

describe("toggleTerminal", () => {
  it("restores a maximized terminal after hiding it", () => {
    layoutStore.setState({ terminalVisible: true, terminalMaximized: true });

    toggleTerminal();
    expect(layoutStore.getState().terminalVisible).toBe(false);
    expect(layoutStore.getState().terminalMaximized).toBe(true);

    toggleTerminal();
    expect(layoutStore.getState().terminalVisible).toBe(true);
    expect(layoutStore.getState().terminalMaximized).toBe(true);
  });

  it("restores a docked terminal after hiding it", () => {
    layoutStore.setState({ terminalVisible: true, terminalMaximized: false });

    toggleTerminal();
    expect(layoutStore.getState().terminalVisible).toBe(false);
    expect(layoutStore.getState().terminalMaximized).toBe(false);

    toggleTerminal();
    expect(layoutStore.getState().terminalVisible).toBe(true);
    expect(layoutStore.getState().terminalMaximized).toBe(false);
  });
});
