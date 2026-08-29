import type { JSX } from "@solidjs/web";
import { layoutStore } from "../../stores/layout-store";
import { settingsStore } from "../../stores/settings-store";
import { explorerStore } from "../../stores/explorer-store";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { GitOutput } from "../terminal/git-output";

const MAX_TAB_LABEL = 24;

const truncateLabel = (name: string): string => {
  if (name.length <= MAX_TAB_LABEL) return name;
  return name.slice(0, MAX_TAB_LABEL - 1) + "\u2026";
};

type MainPanelProps = {
  changesView: JSX.Element;
  historyView: JSX.Element;
  settingsView?: JSX.Element;
  fileView?: JSX.Element;
};

export const MainPanel = (props: MainPanelProps) => {
  const mainView = useStore(layoutStore, (s) => s.mainView);
  const sidebarView = useStore(layoutStore, (s) => s.sidebarView);
  const terminalMaximized = useStore(layoutStore, (s) => s.terminalMaximized);
  const sidebarRight = useStore(settingsStore, (s) => s.settings.ui.sidebarPosition === "right");
  const explorerPath = useStore(explorerStore, (s) => s.selectedPath);
  const diffFile = useStore(repositoryStore, (s) => s.selectedFile);
  const { setMainView } = layoutStore.getState();

  const isFirstTabActive = () => mainView() === "changes" || mainView() === "file";
  const activePath = () => (sidebarView() === "explorer" ? explorerPath() : (diffFile()?.path ?? null));
  const fileName = () => activePath()?.split("/").pop() ?? null;
  const firstTabTitle = () => activePath() ?? undefined;
  const firstTabView = () => (sidebarView() === "explorer" ? "file" : "changes");

  return (
    <div style={{ display: "flex", "flex-direction": "column", height: "100%" }}>
      <div class="main-view-tabs" style={sidebarRight() ? { "flex-direction": "row-reverse" } : undefined}>
        {fileName() && (
          <button
            class={`main-view-tab main-view-tab-primary${isFirstTabActive() ? " active" : ""}`}
            onClick={() => setMainView(firstTabView())}
            title={firstTabTitle()}
          >
            {truncateLabel(fileName()!)}
          </button>
        )}
        {terminalMaximized() && (
          <button
            class={`main-view-tab${mainView() === "terminal" ? " active" : ""}`}
            onClick={() => setMainView("terminal")}
          >
            Terminal
          </button>
        )}
        <div class="main-view-tab-spacer" />
        {terminalMaximized() && (
          <button
            class={`main-view-tab${mainView() === "output" ? " active" : ""}`}
            onClick={() => setMainView("output")}
          >
            Output
          </button>
        )}
        <button
          class={`main-view-tab${mainView() === "history" ? " active" : ""}`}
          onClick={() => setMainView("history")}
        >
          History
        </button>
        <button
          class={`main-view-tab${mainView() === "settings" ? " active" : ""}`}
          onClick={() => setMainView(layoutStore.getState().mainView === "settings" ? "changes" : "settings")}
        >
          Settings
        </button>
      </div>
      <div style={{ flex: 1, "min-height": 0, display: mainView() === "changes" ? undefined : "none" }}>
        {props.changesView}
      </div>
      <div style={{ flex: 1, "min-height": 0, display: mainView() === "history" ? undefined : "none" }}>
        {props.historyView}
      </div>
      {mainView() === "output" && (
        <div style={{ flex: 1, "min-height": 0 }}>
          <GitOutput />
        </div>
      )}
      {mainView() === "settings" && props.settingsView && (
        <div style={{ flex: 1, "min-height": 0 }}>{props.settingsView}</div>
      )}
      {mainView() === "file" && props.fileView && <div style={{ flex: 1, "min-height": 0 }}>{props.fileView}</div>}
    </div>
  );
};
