import type { ReactNode } from "react";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { useExplorerStore } from "../../stores/explorer-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { GitOutput } from "../terminal/git-output";

const MAX_TAB_LABEL = 24;

const truncateLabel = (name: string): string => {
  if (name.length <= MAX_TAB_LABEL) return name;
  return name.slice(0, MAX_TAB_LABEL - 1) + "\u2026";
};

interface MainPanelProps {
  changesView: ReactNode;
  historyView: ReactNode;
  settingsView?: ReactNode;
  fileView?: ReactNode;
}

export const MainPanel = ({ changesView, historyView, settingsView, fileView }: MainPanelProps) => {
  const { mainView, setMainView, sidebarView, terminalMaximized } = useLayoutStore();
  const sidebarRight = useSettingsStore((s) => s.settings.ui.sidebarPosition === "right");
  const explorerPath = useExplorerStore((s) => s.selectedPath);
  const diffFile = useRepositoryStore((s) => s.selectedFile);

  const isFirstTabActive = mainView === "changes" || mainView === "file";
  const activePath = sidebarView === "explorer" ? explorerPath : diffFile?.path ?? null;
  const fileName = activePath?.split("/").pop() ?? null;
  const firstTabTitle = activePath ?? undefined;
  const firstTabView = sidebarView === "explorer" ? "file" : "changes";

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="main-view-tabs" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
        {fileName && (
          <button
            className={`main-view-tab main-view-tab-primary${isFirstTabActive ? " active" : ""}`}
            onClick={() => setMainView(firstTabView)}
            title={firstTabTitle}
          >
            {truncateLabel(fileName)}
          </button>
        )}
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
          className={`main-view-tab${mainView === "history" ? " active" : ""}`}
          onClick={() => setMainView("history")}
        >
          History
        </button>
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
      {mainView === "file" && fileView && (
        <div style={{ flex: 1, minHeight: 0 }}>
          {fileView}
        </div>
      )}
    </div>
  );
};
