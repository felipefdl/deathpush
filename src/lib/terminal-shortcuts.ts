import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";

const isTerminalFocused = (): boolean => {
  const layout = layoutStore.getState();
  if (!layout.terminalVisible || layout.panelTab !== "terminal") return false;
  return !!document.activeElement?.closest(".app-layout-terminal");
};

export const handleTerminalShortcut = (event: KeyboardEvent): boolean => {
  if (!isTerminalFocused()) return false;
  const isMod = event.metaKey || event.ctrlKey;
  if (!isMod || event.altKey) return false;

  const key = event.key.toLowerCase();
  const repo = repositoryStore.getState();

  if (key === "t" && !event.shiftKey) {
    event.preventDefault();
    repo.addTerminalGroup();
    return true;
  }

  if (key === "d") {
    event.preventDefault();
    const groupId = repo.activeGroupId;
    if (groupId === null) return true;
    if (event.shiftKey) {
      repo.splitTerminalVertical(groupId);
    } else {
      repo.splitTerminal(groupId);
    }
    return true;
  }

  if (key === "w" && !event.shiftKey) {
    event.preventDefault();
    const groupId = repo.activeGroupId;
    if (groupId === null) return true;
    const group = repo.terminalGroups.find((item) => item.groupId === groupId);
    if (!group) return true;
    repo.removePane(groupId, group.activePaneId);
    return true;
  }


  return false;
};
