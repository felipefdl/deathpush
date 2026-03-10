import { useCallback, useEffect, useState } from "react";
import { useRepositoryStore } from "../../stores/repository-store";
import * as commands from "../../lib/tauri-commands";
import type { DiscoveredRepo } from "../../lib/tauri-commands";

interface SubReposHeaderProps {
  collapsed: boolean;
  onToggle: () => void;
  count: number;
}

export const SubReposHeader = ({ collapsed, onToggle, count }: SubReposHeaderProps) => (
  <div className="resource-group-header" onClick={onToggle}>
    <span className={`codicon codicon-chevron-down resource-group-chevron ${collapsed ? "collapsed" : ""}`} />
    <span className="resource-group-label">Nested Repositories</span>
    <span className="resource-group-count">{count}</span>
  </div>
);

interface SubReposBodyProps {
  repos: DiscoveredRepo[];
  repoRoot: string;
}

export const SubReposBody = ({ repos, repoRoot }: SubReposBodyProps) => {
  const [branches, setBranches] = useState<Record<string, string | null>>({});

  useEffect(() => {
    const fetchBranches = async () => {
      const map: Record<string, string | null> = {};
      await Promise.all(
        repos.map(async (r) => {
          const fullPath = `${repoRoot}/${r.path}`;
          try {
            map[r.path] = await commands.getRepoBranch(fullPath);
          } catch {
            map[r.path] = null;
          }
        }),
      );
      setBranches(map);
    };
    fetchBranches();
  }, [repos, repoRoot]);

  const handleClick = useCallback(async (repoPath: string) => {
    const fullPath = `${repoRoot}/${repoPath}`;
    try {
      await commands.newWindow(fullPath);
    } catch (err) {
      console.error("Failed to open repository:", err);
    }
  }, [repoRoot]);

  return (
    <div className="resource-group-body">
      {repos.map((repo) => (
        <div
          key={repo.path}
          className="sub-repo-item"
          onClick={() => handleClick(repo.path)}
          title={repo.path}
        >
          <span className="codicon codicon-repo sub-repo-icon" />
          <span className="sub-repo-name">{repo.name}</span>
          {branches[repo.path] && (
            <span className="sub-repo-branch">{branches[repo.path]}</span>
          )}
        </div>
      ))}
    </div>
  );
};

export const useSubRepos = () => {
  const status = useRepositoryStore((s) => s.status);
  const [repos, setRepos] = useState<DiscoveredRepo[]>([]);

  const loadRepos = useCallback(async () => {
    try {
      const discovered = await commands.discoverRepositories();
      setRepos(discovered);
    } catch {
      setRepos([]);
    }
  }, []);

  useEffect(() => {
    if (status?.root) {
      loadRepos();
    } else {
      setRepos([]);
    }
  }, [status?.root, loadRepos]);

  return { repos, loadRepos };
};
