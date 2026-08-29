import { layoutStore } from "../stores/layout-store";
import { repositoryStore } from "../stores/repository-store";

export const toggleTerminal = () => {
  const layout = layoutStore.getState();
  const repo = repositoryStore.getState();

  if (layout.terminalVisible) {
    layout.setTerminalVisible(false);
    return;
  }

  if (repo.terminalGroups.length === 0) {
    repo.addTerminalGroup();
  }
  layout.setTerminalVisible(true);
};
