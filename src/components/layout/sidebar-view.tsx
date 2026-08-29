import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";
import { ScmView } from "../scm/scm-view";
import { ExplorerView } from "../explorer/explorer-view";
import "../../styles/explorer.css";

type SidebarViewProps = {
  onOpenRepository: () => void;
  onCloneRepository: () => void;
};

export const SidebarView = (props: SidebarViewProps) => {
  const sidebarView = useStore(layoutStore, (s) => s.sidebarView);
  const { setSidebarView } = layoutStore.getState();

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div class="sidebar-tabs">
        <button class={`sidebar-tab${sidebarView() === "scm" ? " active" : ""}`} onClick={() => setSidebarView("scm")}>
          Changes
        </button>
        <button
          class={`sidebar-tab${sidebarView() === "explorer" ? " active" : ""}`}
          onClick={() => setSidebarView("explorer")}
        >
          Explorer
        </button>
      </div>
      <div style={{ flex: 1, "min-height": 0, display: sidebarView() === "scm" ? undefined : "none" }}>
        <ScmView
          onOpenRepository={() => props.onOpenRepository()}
          onCloneRepository={() => props.onCloneRepository()}
        />
      </div>
      <div style={{ flex: 1, "min-height": 0, display: sidebarView() === "explorer" ? undefined : "none" }}>
        <ExplorerView onOpenRepository={() => props.onOpenRepository()} />
      </div>
    </div>
  );
};
