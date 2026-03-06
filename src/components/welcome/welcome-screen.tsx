import { useEffect, useState, useCallback } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getRecentProjects, removeRecentProject, type RecentProject } from "../../lib/recent-projects";
import { scanProjectsDirectory, type ProjectInfo } from "../../lib/tauri-commands";
import { useSettingsStore } from "../../stores/settings-store";
import { useThemeStore } from "../../stores/theme-store";

interface WelcomeScreenProps {
  onOpenRepository: () => void;
  onCloneRepository: () => void;
  onSelectProject: (path: string) => void;
}

export const WelcomeScreen = ({ onOpenRepository, onCloneRepository, onSelectProject }: WelcomeScreenProps) => {
  const [recents, setRecents] = useState<RecentProject[]>([]);
  const [workspaceProjects, setWorkspaceProjects] = useState<ProjectInfo[]>([]);
  const projectsSettings = useSettingsStore((s) => s.settings.projects);
  const updateProjects = useSettingsStore((s) => s.updateProjects);
  const themeKind = useThemeStore((s) => s.currentTheme.kind);
  const isDark = themeKind === "dark" || themeKind === "hc-dark";

  useEffect(() => {
    setRecents(getRecentProjects());
  }, []);

  useEffect(() => {
    if (projectsSettings.projectsDirectory) {
      scanProjectsDirectory(projectsSettings.projectsDirectory, projectsSettings.scanDepth)
        .then(setWorkspaceProjects)
        .catch(() => setWorkspaceProjects([]));
    } else {
      setWorkspaceProjects([]);
    }
  }, [projectsSettings.projectsDirectory, projectsSettings.scanDepth]);

  const handleRemoveRecent = useCallback((e: React.MouseEvent, path: string) => {
    e.stopPropagation();
    removeRecentProject(path);
    setRecents(getRecentProjects());
  }, []);

  const handleConfigureWorkspace = useCallback(async () => {
    const selected = await open({ directory: true, title: "Select Git Projects Directory" });
    if (selected) {
      updateProjects({ projectsDirectory: selected });
    }
  }, [updateProjects]);

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
            <div className="welcome-list">
              {recents.length === 0 ? (
                <div className="welcome-list-empty">No recent projects</div>
              ) : (
                recents.map((project) => (
                  <button
                    key={project.path}
                    className="welcome-list-item"
                    onClick={() => onSelectProject(project.path)}
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
            {projectsSettings.projectsDirectory ? (
              <div className="welcome-list">
                {workspaceProjects.length === 0 ? (
                  <div className="welcome-list-empty">No git repositories found</div>
                ) : (
                  workspaceProjects.map((project) => (
                    <button
                      key={project.path}
                      className="welcome-list-item"
                      onClick={() => onSelectProject(project.path)}
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
            ) : (
              <div className="welcome-list">
                <div className="welcome-list-empty">No projects directory configured</div>
              </div>
            )}
            <button className="welcome-configure-btn" onClick={handleConfigureWorkspace}>
              {projectsSettings.projectsDirectory ? "Change Directory..." : "Set Projects Directory..."}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
};
