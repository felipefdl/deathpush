import { createEffect, createMemo, createSignal, For, onSettled, Show, untrack } from "solid-js";
import { getRecentProjects, removeRecentProject, type RecentProject } from "../../lib/recent-projects";
import { scanProjectsDirectory, type ProjectInfo } from "../../lib/tauri-commands";
import { buildMultiRootWorkspaceTree, type WorkspaceTreeNode } from "../../lib/workspace-tree";
import { settingsStore, type WorkspaceEntry } from "../../stores/settings-store";
import { repositoryStore } from "../../stores/repository-store";
import { themeStore } from "../../stores/theme-store";
import { useStore } from "../../lib/use-store";
import { WorkspaceConfigModal } from "../shared/workspace-config-modal";
import { Spinner } from "../ui/spinner";
import { checkForUpdate, downloadAndInstallUpdate } from "../../lib/updater";
import type { Update } from "@tauri-apps/plugin-updater";
import { IS_MACOS, IS_LINUX } from "../../lib/platform";

const MOD_KEY = IS_MACOS ? "\u2318" : "Ctrl+";

const handleListNavKeyDown = (e: KeyboardEvent) => {
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
  e.preventDefault();
  const container = (e.currentTarget as HTMLElement).closest(".welcome-list");
  if (!container) return;
  const focusables = Array.from(
    container.querySelectorAll<HTMLElement>(".welcome-list-item, .welcome-tree-project, .welcome-tree-folder")
  );
  const idx = focusables.indexOf(e.currentTarget as HTMLElement);
  const next = e.key === "ArrowDown" ? idx + 1 : idx - 1;
  focusables[next]?.focus();
};

type WorkspaceFolderProps = {
  node: WorkspaceTreeNode;
  depth: number;
  onSelectProject: (path: string) => void;
};

const WorkspaceFolder = (props: WorkspaceFolderProps) => {
  const [collapsed, setCollapsed] = createSignal(untrack(() => !!props.node.name));

  const sortedChildren = createMemo(() =>
    Array.from(props.node.children.values()).sort((a, b) => a.name.localeCompare(b.name))
  );
  const sortedProjects = createMemo(() => [...props.node.projects].sort((a, b) => a.name.localeCompare(b.name)));

  return (
    <div>
      {props.node.name && (
        <div
          class="welcome-tree-folder"
          style={{ "padding-left": `${12 + props.depth * 16}px` }}
          tabindex={0}
          role="button"
          onClick={() => setCollapsed(!collapsed())}
          onKeyDown={(e) => {
            if (e.key === "ArrowRight" || e.key === "Enter") {
              if (collapsed()) {
                e.preventDefault();
                setCollapsed(false);
                return;
              }
            } else if (e.key === "ArrowLeft") {
              if (!collapsed()) {
                e.preventDefault();
                setCollapsed(true);
                return;
              }
            } else if (e.key === " ") {
              e.preventDefault();
              setCollapsed(!collapsed());
              return;
            }
            handleListNavKeyDown(e);
          }}
        >
          <span class={["codicon", "codicon-chevron-down", "welcome-tree-chevron", { collapsed: collapsed() }]} />
          <span class="codicon codicon-folder" />
          <span class="welcome-tree-folder-name">{props.node.name}</span>
        </div>
      )}
      {!collapsed() && (
        <>
          <For each={sortedChildren()} keyed={(child) => child.name}>
            {(child) => (
              <WorkspaceFolder
                node={child()}
                depth={props.node.name ? props.depth + 1 : props.depth}
                onSelectProject={props.onSelectProject}
              />
            )}
          </For>
          <For each={sortedProjects()} keyed={(project) => project.path}>
            {(project) => (
              <button
                class="welcome-tree-project"
                style={{ "padding-left": `${12 + (props.node.name ? props.depth + 1 : props.depth) * 16}px` }}
                onClick={() => props.onSelectProject(project().path)}
                onKeyDown={handleListNavKeyDown}
              >
                <span class="codicon codicon-repo" />
                <span class="welcome-tree-project-name">{project().name}</span>
              </button>
            )}
          </For>
        </>
      )}
    </div>
  );
};

type WorkspaceTreeProps = {
  projects: ProjectInfo[];
  workspaces: WorkspaceEntry[];
  onSelectProject: (path: string) => void;
};

const WorkspaceTree = (props: WorkspaceTreeProps) => {
  const tree = createMemo(() => buildMultiRootWorkspaceTree(props.projects, props.workspaces));
  return <WorkspaceFolder node={tree()} depth={0} onSelectProject={props.onSelectProject} />;
};

type WelcomeScreenProps = {
  onOpenRepository: () => void;
  onCloneRepository: () => void;
  onSelectProject: (path: string) => void;
};

export const WelcomeScreen = (props: WelcomeScreenProps) => {
  const [recents, setRecents] = createSignal<RecentProject[]>([]);
  const [recentFilter, setRecentFilter] = createSignal("");
  const [recentIndex, setRecentIndex] = createSignal<number | null>(null);
  const [workspaceProjects, setWorkspaceProjects] = createSignal<ProjectInfo[]>([]);
  const [workspaceFilter, setWorkspaceFilter] = createSignal("");
  const [workspaceIndex, setWorkspaceIndex] = createSignal<number | null>(null);
  const [showConfigModal, setShowConfigModal] = createSignal(false);
  const [availableUpdate, setAvailableUpdate] = createSignal<Update | null>(null);
  const [updateProgress, setUpdateProgress] = createSignal<number | null>(null);
  let recentFilterRef: HTMLInputElement | undefined;
  let workspaceFilterRef: HTMLInputElement | undefined;
  let recentListRef: HTMLDivElement | undefined;
  let workspaceListRef: HTMLDivElement | undefined;
  let updating = false;

  const projectsSettings = useStore(settingsStore, (s) => s.settings.projects);
  const { updateProjects } = settingsStore.getState();
  const themeKind = useStore(themeStore, (s) => s.currentTheme.kind);
  const isDark = createMemo(() => themeKind() === "dark");
  const opening = useStore(repositoryStore, (s) => s.operations.has("open-repo"));

  onSettled(() => {
    setRecents(getRecentProjects());
  });

  onSettled(() => {
    const timer = setTimeout(() => {
      void checkForUpdate().then(setAvailableUpdate);
    }, 2000);
    return () => clearTimeout(timer);
  });

  const handleUpdate = () => {
    const update = availableUpdate();
    if (!update || updating) return;
    updating = true;
    setUpdateProgress(0);
    downloadAndInstallUpdate(update, (downloaded, total) => {
      if (total) {
        setUpdateProgress(Math.round((downloaded / total) * 100));
      }
    }).catch(() => {
      setUpdateProgress(null);
      updating = false;
    });
  };

  createEffect(
    () => projectsSettings().workspaces,
    (workspaces) => {
      if (workspaces.length === 0) {
        setWorkspaceProjects([]);
        return;
      }
      let cancelled = false;
      void Promise.all(
        workspaces.map((ws) => scanProjectsDirectory(ws.directory, ws.scanDepth).catch(() => [] as ProjectInfo[]))
      ).then((results) => {
        if (cancelled) return;
        const seen = new Set<string>();
        const merged: ProjectInfo[] = [];
        for (const list of results) {
          for (const p of list) {
            if (!seen.has(p.path)) {
              seen.add(p.path);
              merged.push(p);
            }
          }
        }
        merged.sort((a, b) => a.name.localeCompare(b.name));
        setWorkspaceProjects(merged);
      });
      return () => {
        cancelled = true;
      };
    }
  );

  const filteredRecents = createMemo(() => {
    const filter = recentFilter();
    const list = recents();
    if (!filter) return list;
    const lower = filter.toLowerCase();
    return list.filter((p) => p.name.toLowerCase().includes(lower) || p.path.toLowerCase().includes(lower));
  });

  const filteredWorkspaceProjects = createMemo(() => {
    const filter = workspaceFilter();
    const list = workspaceProjects();
    if (!filter) return list;
    const lower = filter.toLowerCase();
    return list.filter((p) => p.name.toLowerCase().includes(lower) || p.path.toLowerCase().includes(lower));
  });

  createEffect(
    () => recentFilter(),
    () => {
      setRecentIndex(null);
    },
    { defer: true }
  );

  createEffect(
    () => workspaceFilter(),
    () => {
      setWorkspaceIndex(null);
    },
    { defer: true }
  );

  createEffect(
    () => recentIndex(),
    (idx) => {
      if (idx === null || !recentListRef) return;
      const items = recentListRef.querySelectorAll(".welcome-list-item");
      items[idx]?.scrollIntoView({ block: "nearest" });
    }
  );

  createEffect(
    () => workspaceIndex(),
    (idx) => {
      if (idx === null || !workspaceListRef) return;
      const items = workspaceListRef.querySelectorAll(".welcome-list-item");
      items[idx]?.scrollIntoView({ block: "nearest" });
    }
  );

  onSettled(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;
      if (isMod && e.key === "1") {
        e.preventDefault();
        recentFilterRef?.focus();
      }
      if (isMod && e.key === "2") {
        e.preventDefault();
        workspaceFilterRef?.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  const handleRecentKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setRecentIndex((prev) => {
        const max = filteredRecents().length - 1;
        return prev === null ? 0 : Math.min(prev + 1, max);
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setRecentIndex((prev) => (prev === null ? 0 : Math.max(prev - 1, 0)));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const idx = recentIndex();
      const list = filteredRecents();
      if (idx !== null && list[idx]) {
        props.onSelectProject(list[idx].path);
      } else if (list.length > 0) {
        setRecentIndex(0);
      }
    } else if (e.key === "Escape") {
      recentFilterRef?.blur();
      setRecentIndex(null);
    }
  };

  const isTreeView = createMemo(() => {
    const workspaces = projectsSettings().workspaces;
    return (
      (workspaces.length > 1 || workspaces.some((ws) => ws.scanDepth > 1)) &&
      !workspaceFilter() &&
      workspaceIndex() === null
    );
  });

  const handleWorkspaceKeyDown = (e: KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (isTreeView() && workspaceListRef) {
        const first = workspaceListRef.querySelector<HTMLElement>(".welcome-tree-folder, .welcome-tree-project");
        first?.focus();
        return;
      }
      setWorkspaceIndex((prev) => {
        const max = filteredWorkspaceProjects().length - 1;
        return prev === null ? 0 : Math.min(prev + 1, max);
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setWorkspaceIndex((prev) => (prev === null ? 0 : Math.max(prev - 1, 0)));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const idx = workspaceIndex();
      const list = filteredWorkspaceProjects();
      if (idx !== null && list[idx]) {
        props.onSelectProject(list[idx].path);
      } else if (list.length > 0) {
        setWorkspaceIndex(0);
      }
    } else if (e.key === "Escape") {
      workspaceFilterRef?.blur();
      setWorkspaceIndex(null);
    }
  };

  const handleRemoveRecent = (e: MouseEvent, path: string) => {
    e.stopPropagation();
    removeRecentProject(path);
    setRecents(getRecentProjects());
  };

  return (
    <div class="welcome-screen">
      {IS_LINUX ? (
        <div style={{ height: "16px", "flex-shrink": 0 }} />
      ) : (
        <div class="welcome-drag-region" data-tauri-drag-region />
      )}
      <div class="welcome-body">
        <div class={["welcome-logo-wrapper", { "has-update": !!availableUpdate() }]}>
          <img class="welcome-logo" src={isDark() ? "/deathpush-white.png" : "/deathpush-black.png"} alt="DeathPush" />
        </div>
        <div class="welcome-title">DeathPush</div>

        <div class="welcome-actions">
          <button class="welcome-action-btn" onClick={() => props.onOpenRepository()}>
            <span class="codicon codicon-folder-opened" />
            Open Repository
          </button>
          <button class="welcome-action-btn" onClick={() => props.onCloneRepository()}>
            <span class="codicon codicon-cloud-download" />
            Clone Repository
          </button>
        </div>

        <div class="welcome-lists">
          <div class="welcome-list-section">
            <div class="welcome-list-header">Recent</div>
            <div class="welcome-filter">
              <span class="codicon codicon-search welcome-filter-icon" />
              <input
                ref={(el) => {
                  recentFilterRef = el;
                }}
                class="welcome-filter-input"
                type="search"
                placeholder={`Filter recent (${MOD_KEY}1)`}
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck={false}
                data-form-type="other"
                value={recentFilter()}
                onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) =>
                  setRecentFilter(e.currentTarget.value)
                }
                onKeyDown={handleRecentKeyDown}
                onBlur={() => setRecentIndex(null)}
              />
            </div>
            <div
              class="welcome-list"
              ref={(el) => {
                recentListRef = el;
              }}
            >
              {recents().length === 0 ? (
                <div class="welcome-list-empty">No recent projects</div>
              ) : filteredRecents().length === 0 ? (
                <div class="welcome-list-empty">No matching projects</div>
              ) : (
                <For each={filteredRecents()} keyed={(project) => project.path}>
                  {(project, i) => (
                    <div
                      role="button"
                      tabindex={0}
                      class={["welcome-list-item", { selected: recentIndex() === i() }]}
                      onClick={() => props.onSelectProject(project().path)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter" || e.key === " ") {
                          e.preventDefault();
                          props.onSelectProject(project().path);
                        } else {
                          handleListNavKeyDown(e);
                        }
                      }}
                    >
                      <span class="codicon codicon-repo" />
                      <div class="welcome-list-item-info">
                        <div class="welcome-list-item-name">{project().name}</div>
                        <div class="welcome-list-item-path">{project().path}</div>
                      </div>
                      <button
                        class="welcome-list-remove"
                        onClick={(e) => handleRemoveRecent(e, project().path)}
                        title="Remove from recents"
                      >
                        <span class="codicon codicon-close" />
                      </button>
                    </div>
                  )}
                </For>
              )}
            </div>
          </div>

          <div class="welcome-list-section">
            <div class="welcome-list-header">Workspace</div>
            <div class="welcome-filter">
              <span class="codicon codicon-search welcome-filter-icon" />
              <input
                ref={(el) => {
                  workspaceFilterRef = el;
                }}
                class="welcome-filter-input"
                type="search"
                placeholder={`Filter workspace (${MOD_KEY}2)`}
                autocomplete="off"
                autocorrect="off"
                autocapitalize="off"
                spellcheck={false}
                data-form-type="other"
                value={workspaceFilter()}
                onInput={(e: InputEvent & { currentTarget: HTMLInputElement }) =>
                  setWorkspaceFilter(e.currentTarget.value)
                }
                onKeyDown={handleWorkspaceKeyDown}
                onBlur={() => setWorkspaceIndex(null)}
              />
            </div>
            <div
              class="welcome-list"
              ref={(el) => {
                workspaceListRef = el;
              }}
            >
              {projectsSettings().workspaces.length === 0 ? (
                <div class="welcome-list-empty">No workspace directories configured</div>
              ) : workspaceProjects().length === 0 ? (
                <div class="welcome-list-empty">No git repositories found</div>
              ) : filteredWorkspaceProjects().length === 0 ? (
                <div class="welcome-list-empty">No matching projects</div>
              ) : isTreeView() ? (
                <WorkspaceTree
                  projects={filteredWorkspaceProjects()}
                  workspaces={projectsSettings().workspaces}
                  onSelectProject={props.onSelectProject}
                />
              ) : (
                <For each={filteredWorkspaceProjects()} keyed={(project) => project.path}>
                  {(project, i) => (
                    <button
                      class={["welcome-list-item", { selected: workspaceIndex() === i() }]}
                      onClick={() => props.onSelectProject(project().path)}
                      onKeyDown={handleListNavKeyDown}
                    >
                      <span class="codicon codicon-repo" />
                      <div class="welcome-list-item-info">
                        <div class="welcome-list-item-name">{project().name}</div>
                        <div class="welcome-list-item-path">{project().path}</div>
                      </div>
                    </button>
                  )}
                </For>
              )}
            </div>
            <div class="welcome-workspace-footer">
              <button class="welcome-configure-btn" onClick={() => setShowConfigModal(true)}>
                Configure Workspace...
              </button>
            </div>
            <Show when={showConfigModal()}>
              <WorkspaceConfigModal
                onClose={() => setShowConfigModal(false)}
                workspaces={projectsSettings().workspaces}
                onSave={(workspaces) => updateProjects({ workspaces })}
              />
            </Show>
          </div>
        </div>
      </div>
      <div class="welcome-footer">
        <Show when={availableUpdate()}>
          {(update) => (
            <button class="welcome-update-btn" onClick={handleUpdate} disabled={updateProgress() !== null}>
              <span class="codicon codicon-cloud-download" />
              {updateProgress() !== null ? `Updating ${updateProgress()}%` : `Update to v${update().version}`}
            </button>
          )}
        </Show>
        <span class="welcome-version">
          Version {__APP_VERSION__} ({__GIT_HASH__})
        </span>
      </div>
      {opening() && (
        <div class="welcome-opening" aria-live="polite" aria-busy="true">
          <Spinner />
          <span class="welcome-opening-label">Opening repository...</span>
        </div>
      )}
    </div>
  );
};
