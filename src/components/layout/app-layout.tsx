import type { JSX } from "@solidjs/web";
import { layoutStore } from "../../stores/layout-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { TitleBar } from "./title-bar";

type AppLayoutProps = {
  sidebar: JSX.Element;
  main: JSX.Element;
  terminal: JSX.Element;
  statusBar: JSX.Element;
};

export const AppLayout = (props: AppLayoutProps) => {
  const sidebarWidth = useStore(layoutStore, (s) => s.sidebarWidth);
  const terminalVisible = useStore(layoutStore, (s) => s.terminalVisible);
  const terminalHeight = useStore(layoutStore, (s) => s.terminalHeight);
  const terminalMaximized = useStore(layoutStore, (s) => s.terminalMaximized);
  const mainView = useStore(layoutStore, (s) => s.mainView);
  const sidebarPosition = useStore(settingsStore, (s) => s.settings.ui.sidebarPosition);
  const { setSidebarWidth, setTerminalHeight } = layoutStore.getState();

  const handleSidebarMouseDown = (e: MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth();
    const direction = sidebarPosition() === "left" ? 1 : -1;

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

  const handleTerminalMouseDown = (e: MouseEvent) => {
    e.preventDefault();
    const startY = e.clientY;
    const startHeight = terminalHeight();

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

  const terminalInMain = () => terminalVisible() && terminalMaximized() && mainView() === "terminal";
  const terminalInBottom = () => terminalVisible() && !terminalMaximized();

  return (
    <div class="app-layout">
      <TitleBar />
      <div class="app-layout-body">
        {sidebarPosition() === "left" && (
          <>
            <div class="app-layout-sidebar" style={{ width: `${sidebarWidth()}px` }}>
              {props.sidebar}
            </div>
            <div class="app-layout-divider" onMouseDown={handleSidebarMouseDown} />
          </>
        )}
        <div class="app-layout-main-wrapper">
          <div class="app-layout-main" style={terminalInMain() ? { flex: "none", overflow: "visible" } : undefined}>
            {props.main}
          </div>
          <div
            class="app-layout-terminal-divider"
            onMouseDown={handleTerminalMouseDown}
            style={{ display: terminalInBottom() ? undefined : "none" }}
          />
          <div
            style={{
              height: terminalInBottom() ? `${terminalHeight()}px` : undefined,
              flex: terminalInMain() ? 1 : undefined,
              "flex-shrink": terminalInBottom() ? 0 : undefined,
              "min-height": terminalInMain() ? 0 : undefined,
              display: terminalInMain() || terminalInBottom() ? undefined : "none",
            }}
          >
            {props.terminal}
          </div>
        </div>
        {sidebarPosition() === "right" && (
          <>
            <div class="app-layout-divider" onMouseDown={handleSidebarMouseDown} />
            <div class="app-layout-sidebar" style={{ width: `${sidebarWidth()}px` }}>
              {props.sidebar}
            </div>
          </>
        )}
      </div>
      <div class="app-layout-statusbar">{props.statusBar}</div>
    </div>
  );
};
