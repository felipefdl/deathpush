import type { ReactNode } from "react";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { TitleBar } from "./title-bar";

interface AppLayoutProps {
  sidebar: ReactNode;
  main: ReactNode;
  terminal: ReactNode;
  statusBar: ReactNode;
}

export const AppLayout = ({ sidebar, main, terminal, statusBar }: AppLayoutProps) => {
  const { sidebarWidth, setSidebarWidth, terminalVisible, terminalHeight, setTerminalHeight, terminalMaximized, mainView } = useLayoutStore();
  const sidebarPosition = useSettingsStore((s) => s.settings.ui.sidebarPosition);

  const handleSidebarMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth;
    const direction = sidebarPosition === "left" ? 1 : -1;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(200, Math.min(600, startWidth + (moveEvent.clientX - startX) * direction));
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  const handleTerminalMouseDown = (e: React.MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = terminalHeight;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newHeight = Math.max(100, Math.min(600, startHeight - (moveEvent.clientY - startY)));
      setTerminalHeight(newHeight);
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  const terminalInMain = terminalVisible && terminalMaximized && mainView === "terminal";
  const terminalInBottom = terminalVisible && !terminalMaximized;

  return (
    <div className="app-layout">
      <TitleBar />
      <div className="app-layout-body">
        {sidebarPosition === "left" && (
          <>
            <div className="app-layout-sidebar" style={{ width: sidebarWidth }}>
              {sidebar}
            </div>
            <div className="app-layout-divider" onMouseDown={handleSidebarMouseDown} />
          </>
        )}
        <div className="app-layout-main-wrapper">
          <div
            className="app-layout-main"
            style={terminalInMain ? { flex: "none", overflow: "visible" } : undefined}
          >
            {main}
          </div>
          <div
            className="app-layout-terminal-divider"
            onMouseDown={handleTerminalMouseDown}
            style={{ display: terminalInBottom ? undefined : "none" }}
          />
          <div style={{
            height: terminalInBottom ? terminalHeight : undefined,
            flex: terminalInMain ? 1 : undefined,
            flexShrink: terminalInBottom ? 0 : undefined,
            minHeight: terminalInMain ? 0 : undefined,
            display: terminalInMain || terminalInBottom ? undefined : "none",
          }}>
            {terminal}
          </div>
        </div>
        {sidebarPosition === "right" && (
          <>
            <div className="app-layout-divider" onMouseDown={handleSidebarMouseDown} />
            <div className="app-layout-sidebar" style={{ width: sidebarWidth }}>
              {sidebar}
            </div>
          </>
        )}
      </div>
      <div className="app-layout-statusbar">
        {statusBar}
      </div>
    </div>
  );
};
