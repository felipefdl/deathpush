import type { ReactNode } from "react";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { GitOutput } from "../terminal/git-output";

interface MainPanelProps {
  changesView: ReactNode;
  historyView: ReactNode;
  settingsView?: ReactNode;
}

export const MainPanel = ({ changesView, historyView, settingsView }: MainPanelProps) => {
  const { mainView, setMainView, terminalMaximized } = useLayoutStore();
  const sidebarRight = useSettingsStore((s) => s.settings.ui.sidebarPosition === "right");

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="main-view-tabs" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
        <button
          className={`main-view-tab${mainView === "changes" ? " active" : ""}`}
          onClick={() => setMainView("changes")}
        >
          Changes
        </button>
        <button
          className={`main-view-tab${mainView === "history" ? " active" : ""}`}
          onClick={() => setMainView("history")}
        >
          History
        </button>
        {terminalMaximized && (
          <button
            className={`main-view-tab${mainView === "terminal" ? " active" : ""}`}
            onClick={() => setMainView("terminal")}
          >
            Terminal
          </button>
        )}
        <div className="main-view-tab-spacer" />
        {terminalMaximized && (
          <button
            className={`main-view-tab${mainView === "output" ? " active" : ""}`}
            onClick={() => setMainView("output")}
          >
            Output
          </button>
        )}
        <button
          className={`main-view-tab${mainView === "settings" ? " active" : ""}`}
          onClick={() => setMainView(mainView === "settings" ? "changes" : "settings")}
        >
          Settings
        </button>
      </div>
      <div style={{ flex: 1, minHeight: 0, display: mainView === "changes" ? undefined : "none" }}>
        {changesView}
      </div>
      <div style={{ flex: 1, minHeight: 0, display: mainView === "history" ? undefined : "none" }}>
        {historyView}
      </div>
      {mainView === "output" && (
        <div style={{ flex: 1, minHeight: 0 }}>
          <GitOutput />
        </div>
      )}
      {mainView === "settings" && settingsView && (
        <div style={{ flex: 1, minHeight: 0 }}>
          {settingsView}
        </div>
      )}
    </div>
  );
};
