import { useEffect, useState, useCallback, useMemo, useRef } from "react";
import { getRecentProjects, removeRecentProject, type RecentProject } from "../../lib/recent-projects";
import { scanProjectsDirectory, type ProjectInfo } from "../../lib/tauri-commands";
import { buildMultiRootWorkspaceTree, type WorkspaceTreeNode } from "../../lib/workspace-tree";
import { useSettingsStore, type WorkspaceEntry } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";
import { WorkspaceConfigModal } from "../shared/workspace-config-modal";

const handleListNavKeyDown = (e: React.KeyboardEvent) => {
  if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
  e.preventDefault();
  const container = (e.currentTarget as HTMLElement).closest(".welcome-list");
  if (!container) return;
  const focusables = Array.from(
    container.querySelectorAll<HTMLElement>(".welcome-list-item, .welcome-tree-project, .welcome-tree-folder"),
  );
  const idx = focusables.indexOf(e.currentTarget as HTMLElement);
  const next = e.key === "ArrowDown" ? idx + 1 : idx - 1;
  focusables[next]?.focus();
};

interface WorkspaceFolderProps {
  node: WorkspaceTreeNode;
  depth: number;
  onSelectProject: (path: string) => void;
}

const WorkspaceFolder = ({ node, depth, onSelectProject }: WorkspaceFolderProps) => {
  const [collapsed, setCollapsed] = useState(!!node.name);

  const sortedChildren = Array.from(node.children.values()).sort((a, b) => a.name.localeCompare(b.name));
  const sortedProjects = [...node.projects].sort((a, b) => a.name.localeCompare(b.name));

  return (
    <div>
      {node.name && (
        <div
          className="welcome-tree-folder"
          style={{ paddingLeft: 12 + depth * 16 }}
          tabIndex={0}
          role="button"
          onClick={() => setCollapsed(!collapsed)}
          onKeyDown={(e) => {
            if (e.key === "ArrowRight" || e.key === "Enter") {
              if (collapsed) {
                e.preventDefault();
                setCollapsed(false);
                return;
              }
            } else if (e.key === "ArrowLeft") {
              if (!collapsed) {
                e.preventDefault();
                setCollapsed(true);
                return;
              }
            } else if (e.key === " ") {
              e.preventDefault();
              setCollapsed(!collapsed);
              return;
            }
            handleListNavKeyDown(e);
          }}
        >
          <span className={`codicon codicon-chevron-down welcome-tree-chevron ${collapsed ? "collapsed" : ""}`} />
          <span className="codicon codicon-folder" />
          <span className="welcome-tree-folder-name">{node.name}</span>
        </div>
      )}
      {!collapsed && (
        <>
          {sortedChildren.map((child) => (
            <WorkspaceFolder
              key={child.name}
              node={child}
              depth={node.name ? depth + 1 : depth}
              onSelectProject={onSelectProject}
            />
          ))}
          {sortedProjects.map((project) => (
            <button
              key={project.path}
              className="welcome-tree-project"
              style={{ paddingLeft: 12 + (node.name ? depth + 1 : depth) * 16 }}
              onClick={() => onSelectProject(project.path)}
              onKeyDown={handleListNavKeyDown}
            >
              <span className="codicon codicon-repo" />
              <span className="welcome-tree-project-name">{project.name}</span>
            </button>
          ))}
        </>
      )}
    </div>
  );
};

interface WorkspaceTreeProps {
  projects: ProjectInfo[];
  workspaces: WorkspaceEntry[];
  onSelectProject: (path: string) => void;
}

const WorkspaceTree = ({ projects, workspaces, onSelectProject }: WorkspaceTreeProps) => {
  const tree = buildMultiRootWorkspaceTree(projects, workspaces);
  return <WorkspaceFolder node={tree} depth={0} onSelectProject={onSelectProject} />;
};

interface WelcomeScreenProps {
  onOpenRepository: () => void;
  onCloneRepository: () => void;
  onSelectProject: (path: string) => void;
}

const IS_MAC = navigator.platform.toUpperCase().includes("MAC");
const MOD_KEY = IS_MAC ? "\u2318" : "Ctrl+";

export const WelcomeScreen = ({ onOpenRepository, onCloneRepository, onSelectProject }: WelcomeScreenProps) => {
  const [recents, setRecents] = useState<RecentProject[]>([]);
  const [recentFilter, setRecentFilter] = useState("");
  const [recentIndex, setRecentIndex] = useState<number | null>(null);
  const [workspaceProjects, setWorkspaceProjects] = useState<ProjectInfo[]>([]);
  const [workspaceFilter, setWorkspaceFilter] = useState("");
  const [workspaceIndex, setWorkspaceIndex] = useState<number | null>(null);
  const [showConfigModal, setShowConfigModal] = useState(false);
  const recentFilterRef = useRef<HTMLInputElement>(null);
  const workspaceFilterRef = useRef<HTMLInputElement>(null);
  const recentListRef = useRef<HTMLDivElement>(null);
  const workspaceListRef = useRef<HTMLDivElement>(null);
  const projectsSettings = useSettingsStore((s) => s.settings.projects);
  const updateProjects = useSettingsStore((s) => s.updateProjects);
  const themeKind = useThemeStore((s) => s.currentTheme.kind);
  const isDark = themeKind === "dark" || themeKind === "hc-dark";

  useEffect(() => {
    setRecents(getRecentProjects());
  }, []);

  useEffect(() => {
    if (projectsSettings.workspaces.length === 0) {
      setWorkspaceProjects([]);
      return;
    }
    Promise.all(
      projectsSettings.workspaces.map((ws) =>
        scanProjectsDirectory(ws.directory, ws.scanDepth).catch(() => [] as ProjectInfo[]),
      ),
    ).then((results) => {
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
  }, [projectsSettings.workspaces]);

  const filteredRecents = useMemo(() => {
    if (!recentFilter) return recents;
    const lower = recentFilter.toLowerCase();
    return recents.filter((p) => p.name.toLowerCase().includes(lower) || p.path.toLowerCase().includes(lower));
  }, [recents, recentFilter]);

  const filteredWorkspaceProjects = useMemo(() => {
    if (!workspaceFilter) return workspaceProjects;
    const lower = workspaceFilter.toLowerCase();
    return workspaceProjects.filter((p) => p.name.toLowerCase().includes(lower) || p.path.toLowerCase().includes(lower));
  }, [workspaceProjects, workspaceFilter]);

  useEffect(() => { setRecentIndex(null); }, [recentFilter]);
  useEffect(() => { setWorkspaceIndex(null); }, [workspaceFilter]);

  useEffect(() => {
    if (recentIndex === null || !recentListRef.current) return;
    const items = recentListRef.current.querySelectorAll(".welcome-list-item");
    items[recentIndex]?.scrollIntoView({ block: "nearest" });
  }, [recentIndex]);

  useEffect(() => {
    if (workspaceIndex === null || !workspaceListRef.current) return;
    const items = workspaceListRef.current.querySelectorAll(".welcome-list-item");
    items[workspaceIndex]?.scrollIntoView({ block: "nearest" });
  }, [workspaceIndex]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      const isMod = e.metaKey || e.ctrlKey;
      if (isMod && e.key === "1") {
        e.preventDefault();
        recentFilterRef.current?.focus();
      }
      if (isMod && e.key === "2") {
        e.preventDefault();
        workspaceFilterRef.current?.focus();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  const handleRecentKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setRecentIndex((prev) => {
        const max = filteredRecents.length - 1;
        return prev === null ? 0 : Math.min(prev + 1, max);
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setRecentIndex((prev) => {
        return prev === null ? 0 : Math.max(prev - 1, 0);
      });
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (recentIndex !== null && filteredRecents[recentIndex]) {
        onSelectProject(filteredRecents[recentIndex].path);
      } else if (filteredRecents.length > 0) {
        setRecentIndex(0);
      }
    } else if (e.key === "Escape") {
      recentFilterRef.current?.blur();
      setRecentIndex(null);
    }
  }, [filteredRecents, recentIndex, onSelectProject]);

  const isTreeView = (projectsSettings.workspaces.length > 1 || projectsSettings.workspaces.some((ws) => ws.scanDepth > 1)) && !workspaceFilter && workspaceIndex === null;

  const handleWorkspaceKeyDown = useCallback((e: React.KeyboardEvent) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (isTreeView && workspaceListRef.current) {
        const first = workspaceListRef.current.querySelector<HTMLElement>(
          ".welcome-tree-folder, .welcome-tree-project",
        );
        first?.focus();
        return;
      }
      setWorkspaceIndex((prev) => {
        const max = filteredWorkspaceProjects.length - 1;
        return prev === null ? 0 : Math.min(prev + 1, max);
      });
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setWorkspaceIndex((prev) => {
        return prev === null ? 0 : Math.max(prev - 1, 0);
      });
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (workspaceIndex !== null && filteredWorkspaceProjects[workspaceIndex]) {
        onSelectProject(filteredWorkspaceProjects[workspaceIndex].path);
      } else if (filteredWorkspaceProjects.length > 0) {
        setWorkspaceIndex(0);
      }
    } else if (e.key === "Escape") {
      workspaceFilterRef.current?.blur();
      setWorkspaceIndex(null);
    }
  }, [filteredWorkspaceProjects, workspaceIndex, onSelectProject, isTreeView]);

  const handleRemoveRecent = useCallback((e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    removeRecentProject(path);
    setRecents(getRecentProjects());
  }, []);

  return (
    <div className="welcome-screen">
      <div className="welcome-drag-region" data-tauri-drag-region />
      <div className="welcome-body">
        <img
          className="welcome-logo"
          src={isDark ? "/deathpush-white.png" : "/deathpush-black.png"}
          alt="DeathPush"
        />
        <div className="welcome-title">DeathPush</div>

        <div className="welcome-actions">
          <button className="welcome-action-btn" onClick={onOpenRepository}>
            <span className="codicon codicon-folder-opened" />
            Open Repository
          </button>
          <button className="welcome-action-btn" onClick={onCloneRepository}>
            <span className="codicon codicon-cloud-download" />
            Clone Repository
          </button>
        </div>

        <div className="welcome-lists">
          <div className="welcome-list-section">
            <div className="welcome-list-header">Recent</div>
            <div className="welcome-filter">
              <span className="codicon codicon-search welcome-filter-icon" />
              <input
                ref={recentFilterRef}
                className="welcome-filter-input"
                type="search"
                placeholder={`Filter recent (${MOD_KEY}1)`}
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
                data-form-type="other"
                value={recentFilter}
                onChange={(e) => setRecentFilter(e.target.value)}
                onKeyDown={handleRecentKeyDown}
                onBlur={() => setRecentIndex(null)}
              />
            </div>
            <div className="welcome-list" ref={recentListRef}>
              {recents.length === 0 ? (
                <div className="welcome-list-empty">No recent projects</div>
              ) : filteredRecents.length === 0 ? (
                <div className="welcome-list-empty">No matching projects</div>
              ) : (
                filteredRecents.map((project, i) => (
                  <button
                    key={project.path}
                    className={`welcome-list-item${recentIndex === i ? " selected" : ""}`}
                    onClick={() => onSelectProject(project.path)}
                    onKeyDown={handleListNavKeyDown}
                  >
                    <span className="codicon codicon-repo" />
                    <div className="welcome-list-item-info">
                      <div className="welcome-list-item-name">{project.name}</div>
                      <div className="welcome-list-item-path">{project.path}</div>
                    </div>
                    <button
                      className="welcome-list-remove"
                      onClick={(e) => handleRemoveRecent(e, project.path)}
                      title="Remove from recents"
                    >
                      <span className="codicon codicon-close" />
                    </button>
                  </button>
                ))
              )}
            </div>
          </div>

          <div className="welcome-list-section">
            <div className="welcome-list-header">Workspace</div>
            <div className="welcome-filter">
              <span className="codicon codicon-search welcome-filter-icon" />
              <input
                ref={workspaceFilterRef}
                className="welcome-filter-input"
                type="search"
                placeholder={`Filter workspace (${MOD_KEY}2)`}
                autoComplete="off"
                autoCorrect="off"
                autoCapitalize="off"
                spellCheck={false}
                data-form-type="other"
                value={workspaceFilter}
                onChange={(e) => setWorkspaceFilter(e.target.value)}
                onKeyDown={handleWorkspaceKeyDown}
                onBlur={() => setWorkspaceIndex(null)}
              />
            </div>
            <div className="welcome-list" ref={workspaceListRef}>
              {projectsSettings.workspaces.length === 0 ? (
                <div className="welcome-list-empty">No workspace directories configured</div>
              ) : workspaceProjects.length === 0 ? (
                <div className="welcome-list-empty">No git repositories found</div>
              ) : filteredWorkspaceProjects.length === 0 ? (
                <div className="welcome-list-empty">No matching projects</div>
              ) : isTreeView ? (
                <WorkspaceTree
                  projects={filteredWorkspaceProjects}
                  workspaces={projectsSettings.workspaces}
                  onSelectProject={onSelectProject}
                />
              ) : (
                filteredWorkspaceProjects.map((project, i) => (
                  <button
                    key={project.path}
                    className={`welcome-list-item${workspaceIndex === i ? " selected" : ""}`}
                    onClick={() => onSelectProject(project.path)}
                    onKeyDown={handleListNavKeyDown}
                  >
                    <span className="codicon codicon-repo" />
                    <div className="welcome-list-item-info">
                      <div className="welcome-list-item-name">{project.name}</div>
                      <div className="welcome-list-item-path">{project.path}</div>
                    </div>
                  </button>
                ))
              )}
            </div>
            <div className="welcome-workspace-footer">
              <button className="welcome-configure-btn" onClick={() => setShowConfigModal(true)}>
                Configure Workspace...
              </button>
            </div>
            {showConfigModal && (
              <WorkspaceConfigModal
                onClose={() => setShowConfigModal(false)}
                workspaces={projectsSettings.workspaces}
                onSave={(workspaces) => updateProjects({ workspaces })}
              />
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
