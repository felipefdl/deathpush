import { createSignal, For } from "solid-js";

import type { StashEntry, FileDiffWithHunks } from "../../lib/git-types";
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

export const StashBody = (props: StashBodyProps) => {
  const { showStash } = useStash();
  const [expandedStash, setExpandedStash] = createSignal<number | null>(null);
  const [stashDiff, setStashDiff] = createSignal<FileDiffWithHunks | null>(null);

  const handleShow = async (index: number) => {
    if (expandedStash() === index) {
      setExpandedStash(null);
      setStashDiff(null);
      return;
    }
    const result = await showStash(index);
    if (result) {
      setStashDiff(result);
      setExpandedStash(index);
    }
  };

  return (
    <div class="resource-group-body">
      <For each={props.stashes} keyed={(entry) => entry.index}>
        {(entry) => (
          <div>
            <StashEntryRow
              entry={entry()}
              onApply={props.onApply}
              onPop={props.onPop}
              onDrop={props.onDrop}
              onShow={handleShow}
            />
            {expandedStash() === entry().index && stashDiff() && (
              <div class="stash-diff-preview">
                <For each={stashDiff()!.hunks} keyed={false}>
                  {(hunk) => (
                    <div class="stash-diff-hunk">
                      <div class="stash-diff-header">{hunk().header}</div>
                      <For each={hunk().lines} keyed={false}>
                        {(line) => (
                          <div class={`stash-diff-line stash-diff-line-${line().lineType}`}>
                            {line().lineType === "add" ? "+" : line().lineType === "remove" ? "-" : " "}
                            {line().content}
                          </div>
                        )}
                      </For>
                    </div>
                  )}
                </For>
                {stashDiff()!.hunks.length === 0 && <div class="stash-diff-empty">No diff available</div>}
              </div>
            )}
          </div>
        )}
      </For>
    </div>
  );
};

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
