import { createEffect, createSignal, For } from "solid-js";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import * as commands from "../../lib/tauri-commands";
import type { NestedRepository } from "../../lib/tauri-commands";

type SubReposHeaderProps = {
  collapsed: boolean;
  onToggle: () => void;
  count: number;
};

export const SubReposHeader = (props: SubReposHeaderProps) => (
  <div class="resource-group-header" onClick={() => props.onToggle()}>
    <span class={`codicon codicon-chevron-down resource-group-chevron ${props.collapsed ? "collapsed" : ""}`} />
    <span class="resource-group-label">Nested Repositories</span>
    <span class="resource-group-count">{props.count}</span>
  </div>
);

type SubReposBodyProps = {
  repos: NestedRepository[];
  repoRoot: string;
};

export const SubReposBody = (props: SubReposBodyProps) => {
  const handleClick = async (repoPath: string) => {
    const fullPath = `${props.repoRoot}/${repoPath}`;
    try {
      await commands.newWindow(fullPath);
    } catch (err) {
      console.error("Failed to open repository:", err);
    }
  };

  return (
    <div class="resource-group-body">
      <For each={props.repos} keyed={(repo) => repo.path}>
        {(repo) => (
          <div class="sub-repo-item" onClick={() => handleClick(repo().path)} title={repo().path}>
            <span class="codicon codicon-repo sub-repo-icon" />
            <span class="sub-repo-name">{repo().name}</span>
            {repo().branch && <span class="sub-repo-branch">{repo().branch}</span>}
          </div>
        )}
      </For>
    </div>
  );
};

export const useSubRepos = () => {
  const status = useStore(repositoryStore, (s) => s.status);
  const [repos, setRepos] = createSignal<NestedRepository[]>([]);

  const loadRepos = async () => {
    try {
      const discovered = await commands.discoverNestedRepositories();
      setRepos(discovered);
    } catch {
      setRepos([]);
    }
  };

  createEffect(
    () => status()?.root,
    (root) => {
      if (root) {
        void loadRepos();
      } else {
        setRepos([]);
      }
    }
  );

  return { repos, loadRepos };
};
