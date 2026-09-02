import { createSignal, For } from "solid-js";

import type { StashEntry } from "../../lib/git-types";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { useStash } from "../../hooks/use-stash";
import { StashEntryRow } from "./stash-entry";

type StashHeaderProps = {
  collapsed: boolean;
  onToggle: () => void;
  count: number;
};

export const StashHeader = (props: StashHeaderProps) => (
  <div class="resource-group-header" onClick={() => props.onToggle()}>
    <span class={`codicon codicon-chevron-down resource-group-chevron ${props.collapsed ? "collapsed" : ""}`} />
    <span class="resource-group-label">Stashes</span>
    <span class="resource-group-count">{props.count}</span>
  </div>
);

type StashBodyProps = {
  stashes: StashEntry[];
  onApply: (index: number) => void;
  onPop: (index: number) => void;
  onDrop: (index: number) => void;
};

export const StashBody = (props: StashBodyProps) => (
  <div class="resource-group-body">
    <For each={props.stashes} keyed={(entry) => entry.index}>
      {(entry) => <StashEntryRow entry={entry()} onApply={props.onApply} onPop={props.onPop} onDrop={props.onDrop} />}
    </For>
  </div>
);

export const StashView = () => {
  const [collapsed, setCollapsed] = createSignal(false);
  const stashes = useStore(repositoryStore, (s) => s.stashes);
  const status = useStore(repositoryStore, (s) => s.status);
  const { applyStash, popStash, dropStash } = useStash();

  return (
    <>
      {status() && stashes().length > 0 ? (
        <div class="resource-group">
          <StashHeader collapsed={collapsed()} onToggle={() => setCollapsed(!collapsed())} count={stashes().length} />
          {!collapsed() && <StashBody stashes={stashes()} onApply={applyStash} onPop={popStash} onDrop={dropStash} />}
        </div>
      ) : null}
    </>
  );
};
