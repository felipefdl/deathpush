import { beforeEach, describe, expect, it } from "vite-plus/test";
import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";
import { closeExitedTerminalPane } from "./close-exited-terminal-pane";

beforeEach(() => {
  layoutStore.setState({ terminalVisible: true, terminalMaximized: false });
  repositoryStore.setState({
    terminalGroups: [
      { groupId: 1, panes: [{ paneId: 1, name: "Terminal 1" }], activePaneId: 1, splitDirection: "horizontal" },
    ],
    activeGroupId: 1,
    terminalIdCounter: 1,
  });
});

describe("closeExitedTerminalPane", () => {
  it("removes the exited pane when other panes remain", () => {
    repositoryStore.getState().splitTerminal(1);

    closeExitedTerminalPane(2);

    const group = repositoryStore.getState().terminalGroups[0];
    expect(group.panes).toHaveLength(1);
    expect(group.panes[0].paneId).toBe(1);
    expect(layoutStore.getState().terminalVisible).toBe(true);
  });

  it("removes the exited tab when other terminal tabs remain", () => {
    repositoryStore.getState().addTerminalGroup();

    closeExitedTerminalPane(1);

    expect(repositoryStore.getState().terminalGroups).toHaveLength(1);
    expect(repositoryStore.getState().terminalGroups[0].groupId).toBe(2);
    expect(layoutStore.getState().terminalVisible).toBe(true);
  });

  it("closes the panel when the last terminal tab exits", () => {
    closeExitedTerminalPane(1);

    expect(repositoryStore.getState().terminalGroups).toHaveLength(0);
    expect(repositoryStore.getState().activeGroupId).toBeNull();
    expect(layoutStore.getState().terminalVisible).toBe(false);
  });
});
