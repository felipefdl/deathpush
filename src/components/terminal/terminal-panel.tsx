import { useCallback, useEffect, useState } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import { useLayoutStore } from "../../stores/layout-store";
import { useSettingsStore } from "../../stores/settings-store";
import { toggleTerminal } from "../../lib/toggle-terminal";
import { GitOutput } from "./git-output";
import { TerminalGroupView } from "./terminal-group-view";
import "../../styles/terminal.css";

export const TerminalPanel = () => {
  const {
    terminalGroups,
    activeGroupId,
    addTerminalGroup,
    removeTerminalGroup,
    setActiveGroup,
    splitTerminal,
    splitTerminalVertical,
    removePane,
    setActivePaneInGroup,
  } = useRepositoryStore();
  const { panelTab, setPanelTab, toggleTerminalMaximized, terminalMaximized } = useLayoutStore();
  const sidebarRight = useSettingsStore((s) => s.settings.ui.sidebarPosition === "right");
  const [sidebarWidth, setSidebarWidth] = useState(160);

  const handleSidebarMouseDown = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth;
    const direction = useSettingsStore.getState().settings.ui.sidebarPosition === "right" ? 1 : -1;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const newWidth = Math.max(100, Math.min(400, startWidth + (moveEvent.clientX - startX) * direction));
      setSidebarWidth(newWidth);
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [sidebarWidth]);

  useEffect(() => {
    const state = useRepositoryStore.getState();
    if (state.terminalGroups.length === 0) {
      state.addTerminalGroup();
    }
  }, []);

  const isTerminal = terminalMaximized || panelTab === "terminal";
  const totalPanes = terminalGroups.reduce((sum, g) => sum + g.panes.length, 0);
  const showSidebar = isTerminal && (terminalMaximized || totalPanes > 1);

  const sidebarList = (
    <div className="terminal-sidebar-list">
      {terminalGroups.map((group) => (
        <div key={group.groupId} className="terminal-sidebar-group">
          {group.panes.map((pane) => (
            <div
              key={pane.paneId}
              className={`terminal-sidebar-item ${group.groupId === activeGroupId && pane.paneId === group.activePaneId ? "active" : ""}`}
              onClick={() => {
                setActiveGroup(group.groupId);
                setActivePaneInGroup(group.groupId, pane.paneId);
              }}
            >
              <span className="codicon codicon-terminal terminal-sidebar-icon" />
              <span className="terminal-sidebar-name">{pane.name}</span>
              <div className="terminal-sidebar-hover-actions">
                <button
                  className="terminal-sidebar-action-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    splitTerminal(group.groupId);
                  }}
                  title="Split Horizontally"
                >
                  <span className="codicon codicon-split-horizontal" />
                </button>
                <button
                  className="terminal-sidebar-action-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    splitTerminalVertical(group.groupId);
                  }}
                  title="Split Vertically"
                >
                  <span className="codicon codicon-split-vertical" />
                </button>
                <button
                  className="terminal-sidebar-action-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    if (group.panes.length > 1) {
                      removePane(group.groupId, pane.paneId);
                    } else {
                      removeTerminalGroup(group.groupId);
                    }
                  }}
                  title="Kill Terminal"
                >
                  <span className="codicon codicon-trash" />
                </button>
              </div>
            </div>
          ))}
        </div>
      ))}
    </div>
  );

  return (
    <div className="terminal-panel">
      {!terminalMaximized && (
        <div className="terminal-panel-header" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
          <div className="panel-tabs" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
            <div
              className={`panel-tab ${!isTerminal ? "active" : ""}`}
              onClick={() => setPanelTab("git-output")}
            >
              Output
            </div>
            <div
              className={`panel-tab ${isTerminal ? "active" : ""}`}
              onClick={() => setPanelTab("terminal")}
            >
              Terminal
            </div>
          </div>
          {isTerminal && (
            <div className="terminal-header-actions" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
              <button className="terminal-panel-btn" onClick={addTerminalGroup} title="New Terminal">
                <span className="codicon codicon-plus" />
              </button>
              <span className="terminal-header-separator" />
              <button
                className="terminal-panel-btn"
                onClick={() => { if (activeGroupId !== null) splitTerminal(activeGroupId); }}
                title="Split Terminal Horizontally"
              >
                <span className="codicon codicon-split-horizontal" />
              </button>
              <button
                className="terminal-panel-btn"
                onClick={() => { if (activeGroupId !== null) splitTerminalVertical(activeGroupId); }}
                title="Split Terminal Vertically"
              >
                <span className="codicon codicon-split-vertical" />
              </button>
              <button
                className="terminal-panel-btn"
                onClick={toggleTerminalMaximized}
                title="Maximize Panel Size"
              >
                <span className="codicon codicon-chrome-maximize" />
              </button>
              <button className="terminal-panel-btn" onClick={() => toggleTerminal()} title="Close Panel">
                <span className="codicon codicon-close" />
              </button>
            </div>
          )}
        </div>
      )}
      <div className="terminal-panel-body" style={sidebarRight ? { flexDirection: "row-reverse" } : undefined}>
        <div className="terminal-panel-content">
          <div className="terminal-panel-main" style={{ display: isTerminal ? undefined : "none" }}>
            {terminalGroups.map((group) => (
              <TerminalGroupView
                key={group.groupId}
                group={group}
                isActive={isTerminal && group.groupId === activeGroupId}
              />
            ))}
          </div>
          {!terminalMaximized && (
            <div className="terminal-panel-main" style={{ display: isTerminal ? "none" : undefined }}>
              <GitOutput />
            </div>
          )}
        </div>
        {showSidebar && (
          <>
          <div className="terminal-sidebar-divider" onMouseDown={handleSidebarMouseDown} />
          <div className="terminal-sidebar" style={{ width: sidebarWidth }}>
            {terminalMaximized && (
              <div className="terminal-sidebar-actions">
                <div className="terminal-sidebar-actions-left">
                  <button className="terminal-panel-btn" onClick={addTerminalGroup} title="New Terminal">
                    <span className="codicon codicon-plus" />
                  </button>
                  <button
                    className="terminal-panel-btn"
                    onClick={() => { if (activeGroupId !== null) splitTerminal(activeGroupId); }}
                    title="Split Terminal Horizontally"
                  >
                    <span className="codicon codicon-split-horizontal" />
                  </button>
                  <button
                    className="terminal-panel-btn"
                    onClick={() => { if (activeGroupId !== null) splitTerminalVertical(activeGroupId); }}
                    title="Split Terminal Vertically"
                  >
                    <span className="codicon codicon-split-vertical" />
                  </button>
                  <button
                    className="terminal-panel-btn"
                    onClick={toggleTerminalMaximized}
                    title="Restore Panel Size"
                  >
                    <span className="codicon codicon-chrome-restore" />
                  </button>
                </div>
              </div>
            )}
            {sidebarList}
          </div>
          </>
        )}
      </div>
    </div>
  );
};
