import { useLayoutStore } from "../../stores/layout-store";
import { ScmView } from "../scm/scm-view";
import { ExplorerView } from "../explorer/explorer-view";
import "../../styles/explorer.css";

interface SidebarViewProps {
  onOpenRepository: () => void;
  onCloneRepository: () => void;
}

export const SidebarView = ({ onOpenRepository, onCloneRepository }: SidebarViewProps) => {
  const { sidebarView, setSidebarView } = useLayoutStore();

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="sidebar-tabs">
        <button
          className={`sidebar-tab${sidebarView === "scm" ? " active" : ""}`}
          onClick={() => setSidebarView("scm")}
        >
          Changes
        </button>
        <button
          className={`sidebar-tab${sidebarView === "explorer" ? " active" : ""}`}
          onClick={() => setSidebarView("explorer")}
        >
          Explorer
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: sidebarView === "scm" ? undefined : "none" }}>
        <ScmView onOpenRepository={onOpenRepository} onCloneRepository={() => onCloneRepository()} />
      </div>
      <div style={{ flex: 1, minHeight: 0, display: sidebarView === "explorer" ? undefined : "none" }}>
        <ExplorerView onOpenRepository={onOpenRepository} />
      </div>
    </div>
  );
};
