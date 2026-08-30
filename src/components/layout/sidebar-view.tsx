import { createEffect, createSignal } from "solid-js";
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
  const [explorerMounted, setExplorerMounted] = createSignal(sidebarView() === "explorer");

  createEffect(
    () => sidebarView() === "explorer",
    (isExplorerActive) => {
      if (isExplorerActive) setExplorerMounted(true);
    }
  );

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
      <div hidden={sidebarView() !== "scm"} style={{ flex: 1, "min-height": 0 }}>
        <ScmView
          onOpenRepository={() => props.onOpenRepository()}
          onCloneRepository={() => props.onCloneRepository()}
        />
      </div>
      {explorerMounted() && (
        <div hidden={sidebarView() !== "explorer"} style={{ flex: 1, "min-height": 0 }}>
          <ExplorerView onOpenRepository={() => props.onOpenRepository()} />
        </div>
      )}
    </div>
  );
};
