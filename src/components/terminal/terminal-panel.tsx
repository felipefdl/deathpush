import { createMemo, createSignal, For, onSettled } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { layoutStore } from "../../stores/layout-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { toggleTerminal } from "../../lib/toggle-terminal";
import { GitOutput } from "./git-output";
import { requestTerminalFocus, retainTerminalButtonFocus } from "./terminal-instance";
import { TerminalGroupView } from "./terminal-group-view";
import "../../styles/terminal.css";

export const shouldShowTerminalSidebar = (paneCount: number): boolean => paneCount > 1;

export const TerminalPanel = () => {
  const terminalGroups = useStore(repositoryStore, (s) => s.terminalGroups);
  const activeGroupId = useStore(repositoryStore, (s) => s.activeGroupId);
  const panelTab = useStore(layoutStore, (s) => s.panelTab);
  const terminalMaximized = useStore(layoutStore, (s) => s.terminalMaximized);
  const sidebarRight = useStore(settingsStore, (s) => s.settings.ui.sidebarPosition === "right");
  const [sidebarWidth, setSidebarWidth] = createSignal(160);

  const handleSidebarMouseDown = (e: MouseEvent) => {
    e.preventDefault();
    const startX = e.clientX;
    const startWidth = sidebarWidth();
    const direction = settingsStore.getState().settings.ui.sidebarPosition === "right" ? 1 : -1;

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
  };

  onSettled(() => {
    const state = repositoryStore.getState();
    if (state.terminalGroups.length === 0) {
      state.addTerminalGroup();
    }
  });

  const isTerminal = createMemo(() => panelTab() === "terminal");
  const totalPanes = createMemo(() => terminalGroups().reduce((sum, group) => sum + group.panes.length, 0));
  const showSidebar = createMemo(() => isTerminal() && shouldShowTerminalSidebar(totalPanes()));
  const headerStyle = () => (sidebarRight() ? { "flex-direction": "row-reverse" as const } : undefined);

  const splitActive = (vertical: boolean) => {
    const groupId = repositoryStore.getState().activeGroupId;
    if (groupId === null) return;
    const repo = repositoryStore.getState();
    if (vertical) repo.splitTerminalVertical(groupId);
    else repo.splitTerminal(groupId);
  };

  return (
    <div class="terminal-panel">
      <div class="terminal-panel-header" style={headerStyle()}>
        <div class="panel-tabs" style={headerStyle()}>
          <div
            class={["panel-tab", { active: !isTerminal() }]}
            onClick={() => layoutStore.getState().setPanelTab("git-output")}
          >
            Output
          </div>
          <div
            class={["panel-tab", { active: isTerminal() }]}
            onClick={() => {
              layoutStore.getState().setPanelTab("terminal");
              requestAnimationFrame(() => requestTerminalFocus());
            }}
          >
            Terminal
          </div>
        </div>
        {(isTerminal() || terminalMaximized()) && (
          <div class="terminal-header-actions" style={headerStyle()}>
            {isTerminal() && (
              <>
                <button
                  class="terminal-panel-btn"
                  onMouseDown={retainTerminalButtonFocus}
                  onClick={() => repositoryStore.getState().addTerminalGroup()}
                  title="New Terminal"
                >
                  <span class="codicon codicon-plus" />
                </button>
                <span class="terminal-header-separator" />
                <button
                  class="terminal-panel-btn"
                  onMouseDown={retainTerminalButtonFocus}
                  onClick={() => splitActive(false)}
                  title="Split Terminal Horizontally"
                >
                  <span class="codicon codicon-split-horizontal" />
                </button>
                <button
                  class="terminal-panel-btn"
                  onMouseDown={retainTerminalButtonFocus}
                  onClick={() => splitActive(true)}
                  title="Split Terminal Vertically"
                >
                  <span class="codicon codicon-split-vertical" />
                </button>
              </>
            )}
            <button
              class="terminal-panel-btn"
              onMouseDown={retainTerminalButtonFocus}
              onClick={() => {
                layoutStore.getState().toggleTerminalMaximized();
                requestAnimationFrame(() => requestTerminalFocus());
              }}
              title={terminalMaximized() ? "Restore Panel Size" : "Maximize Panel Size"}
            >
              <span class={`codicon ${terminalMaximized() ? "codicon-chrome-restore" : "codicon-chrome-maximize"}`} />
            </button>
            <button
              class="terminal-panel-btn"
              onMouseDown={retainTerminalButtonFocus}
              onClick={() => toggleTerminal()}
              title="Close Panel"
            >
              <span class="codicon codicon-close" />
            </button>
          </div>
        )}
      </div>
      <div class="terminal-panel-body" style={headerStyle()}>
        <div class="terminal-panel-content">
          <div class={`terminal-panel-main${!isTerminal() ? " is-inactive" : ""}`}>
            <For each={terminalGroups()} keyed={(group) => group.groupId}>
              {(group) => (
                <TerminalGroupView
                  group={group()}
                  isActive={group().groupId === activeGroupId()}
                  isFocused={isTerminal() && group().groupId === activeGroupId()}
                />
              )}
            </For>
          </div>
          <div class={`terminal-panel-main${isTerminal() ? " is-inactive" : ""}`}>
            <GitOutput />
          </div>
        </div>
        {showSidebar() && (
          <>
            <div class="terminal-sidebar-divider" onMouseDown={handleSidebarMouseDown} />
            <div class="terminal-sidebar" style={{ width: `${sidebarWidth()}px` }}>
              <div class="terminal-sidebar-list">
                <For each={terminalGroups()} keyed={(group) => group.groupId}>
                  {(group) => (
                    <div class="terminal-sidebar-group">
                      <For each={group().panes} keyed={(pane) => pane.paneId}>
                        {(pane) => (
                          <div
                            class={[
                              "terminal-sidebar-item",
                              {
                                active: group().groupId === activeGroupId() && pane().paneId === group().activePaneId,
                              },
                            ]}
                            onClick={() => {
                              const repo = repositoryStore.getState();
                              repo.setActiveGroup(group().groupId);
                              repo.setActivePaneInGroup(group().groupId, pane().paneId);
                            }}
                          >
                            <span class="codicon codicon-terminal terminal-sidebar-icon" />
                            <span class="terminal-sidebar-name">{pane().name}</span>
                            <div class="terminal-sidebar-hover-actions">
                              <button
                                class="terminal-sidebar-action-btn"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  repositoryStore.getState().splitTerminal(group().groupId);
                                }}
                                title="Split Horizontally"
                              >
                                <span class="codicon codicon-split-horizontal" />
                              </button>
                              <button
                                class="terminal-sidebar-action-btn"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  repositoryStore.getState().splitTerminalVertical(group().groupId);
                                }}
                                title="Split Vertically"
                              >
                                <span class="codicon codicon-split-vertical" />
                              </button>
                              <button
                                class="terminal-sidebar-action-btn"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  const current = group();
                                  if (current.panes.length > 1) {
                                    repositoryStore.getState().removePane(current.groupId, pane().paneId);
                                  } else {
                                    repositoryStore.getState().removeTerminalGroup(current.groupId);
                                  }
                                }}
                                title="Kill Terminal"
                              >
                                <span class="codicon codicon-trash" />
                              </button>
                            </div>
                          </div>
                        )}
                      </For>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </>
        )}
      </div>
    </div>
  );
};
