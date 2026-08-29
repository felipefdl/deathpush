import { afterEach, beforeEach, describe, expect, it } from "vite-plus/test";
import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";
import { handleTerminalShortcut } from "./terminal-shortcuts";

const press = (key: string, shiftKey = false): KeyboardEvent => {
  const event = new KeyboardEvent("keydown", {
    key,
    metaKey: true,
    shiftKey,
    bubbles: true,
    cancelable: true,
  });
  handleTerminalShortcut(event);
  return event;
};

describe("handleTerminalShortcut", () => {
  let terminalEl: HTMLDivElement;

  beforeEach(() => {
    layoutStore.setState({ terminalVisible: true, panelTab: "terminal" });
    repositoryStore.setState({
      terminalGroups: [
        { groupId: 1, panes: [{ paneId: 1, name: "Terminal 1" }], activePaneId: 1, splitDirection: "horizontal" },
      ],
      activeGroupId: 1,
      terminalIdCounter: 1,
    });
    terminalEl = document.createElement("div");
    terminalEl.className = "app-layout-terminal";
    terminalEl.tabIndex = 0;
    document.body.appendChild(terminalEl);
    terminalEl.focus();
  });

  afterEach(() => {
    terminalEl.remove();
  });

  it("opens a new terminal tab with Cmd+T", () => {
    press("t");
    const state = repositoryStore.getState();
    expect(state.terminalGroups).toHaveLength(2);
    expect(state.activeGroupId).toBe(2);
  });

  it("splits horizontally with Cmd+D", () => {
    press("d");
    const group = repositoryStore.getState().terminalGroups[0];
    expect(group.panes).toHaveLength(2);
    expect(group.splitDirection).toBe("horizontal");
  });

  it("splits vertically with Cmd+Shift+D", () => {
    press("D", true);
    const group = repositoryStore.getState().terminalGroups[0];
    expect(group.panes).toHaveLength(2);
    expect(group.splitDirection).toBe("vertical");
  });

  it("closes the active split with Cmd+W", () => {
    press("d");
    press("w");
    const group = repositoryStore.getState().terminalGroups[0];
    expect(group.panes).toHaveLength(1);
  });

  it("closes the active terminal tab with Cmd+W", () => {
    press("t");
    expect(repositoryStore.getState().terminalGroups).toHaveLength(2);
    press("w");
    expect(repositoryStore.getState().terminalGroups).toHaveLength(1);
    expect(repositoryStore.getState().activeGroupId).toBe(1);
  });

  it("ignores the shortcuts when the terminal is not focused", () => {
    terminalEl.blur();
    document.body.focus();
    press("t");
    press("d");
    expect(repositoryStore.getState().terminalGroups).toHaveLength(1);
    expect(repositoryStore.getState().terminalGroups[0].panes).toHaveLength(1);
  });
});
