import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";

export const closeExitedTerminalPane = (paneId: number): void => {
  const repo = repositoryStore.getState();
  const group = repo.terminalGroups.find((item) => item.panes.some((pane) => pane.paneId === paneId));
  if (!group) return;

  const lastTab = repo.terminalGroups.length === 1 && group.panes.length === 1;
  if (lastTab) {
    repositoryStore.setState({ terminalGroups: [], activeGroupId: null });
    layoutStore.getState().setTerminalVisible(false);
    return;
  }

  repo.removePane(group.groupId, paneId);
};
