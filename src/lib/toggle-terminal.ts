import { useLayoutStore } from "../stores/layout-store";
import { useRepositoryStore } from "../stores/repository-store";

export const toggleTerminal = () => {
  const layout = useLayoutStore.getState();
  const repo = useRepositoryStore.getState();

  if (layout.terminalVisible) {
    layout.setTerminalVisible(false);
    if (layout.terminalMaximized) {
      layout.setTerminalMaximized(false);
      layout.setMainView("changes");
    }
    return;
  }

  if (repo.terminalGroups.length === 0) {
    repo.addTerminalGroup();
  }
  layout.setTerminalVisible(true);
};
