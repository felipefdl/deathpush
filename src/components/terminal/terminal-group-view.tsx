import { createEffect, createSignal, For } from "solid-js";
import { repositoryStore, type TerminalGroup } from "../../stores/repository-store";
import { TerminalInstance } from "./terminal-instance";

type TerminalGroupViewProps = {
  group: TerminalGroup;
  isActive: boolean;
  isFocused: boolean;
};

export const TerminalGroupView = (props: TerminalGroupViewProps) => {
  const [flexValues, setFlexValues] = createSignal<number[]>(
    props.group.panes.map(() => 1),
    { ownedWrite: true }
  );
  let containerEl: HTMLDivElement | undefined;

  createEffect(
    () => props.group.panes.length,
    (count) => {
      setFlexValues((prev) => {
        if (prev.length === count) return prev;
        if (count < prev.length) {
          return Array.from({ length: count }, () => 1);
        }
        return Array.from({ length: count }, (_, i) => prev[i] ?? 1);
      });
    }
  );

  const isVertical = () => props.group.splitDirection === "vertical";

  const handleDividerMouseDown = (e: MouseEvent, dividerIndex: number) => {
    e.preventDefault();
    const container = containerEl;
    if (!container) return;

    const paneEls = container.querySelectorAll<HTMLElement>(".terminal-split-pane");
    const firstPane = paneEls[dividerIndex];
    const secondPane = paneEls[dividerIndex + 1];
    if (!firstPane || !secondPane) return;

    const vertical = props.group.splitDirection === "vertical";
    const startPos = vertical ? e.clientY : e.clientX;
    const firstRect = firstPane.getBoundingClientRect();
    const secondRect = secondPane.getBoundingClientRect();
    const firstStartSize = vertical ? firstRect.height : firstRect.width;
    const secondStartSize = vertical ? secondRect.height : secondRect.width;
    const totalSize = firstStartSize + secondStartSize;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const currentPos = vertical ? moveEvent.clientY : moveEvent.clientX;
      const delta = currentPos - startPos;
      const newFirstSize = Math.max(80, firstStartSize + delta);
      const newSecondSize = Math.max(80, secondStartSize - delta);
      const clampedFirst = totalSize - newSecondSize < 80 ? 80 : newFirstSize;
      const clampedSecond = totalSize - clampedFirst;

      setFlexValues((prev) => {
        const next = [...prev];
        next[dividerIndex] = clampedFirst / totalSize;
        next[dividerIndex + 1] = clampedSecond / totalSize;
        return next;
      });
    };

    const handleMouseUp = () => {
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
    };

    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  return (
    <div
      ref={(el) => (containerEl = el)}
      class="terminal-group-view"
      style={{
        display: props.isActive ? "flex" : "none",
        "flex-direction": isVertical() ? "column" : "row",
      }}
    >
      <For each={props.group.panes} keyed={(pane) => pane.paneId}>
        {(pane, i) => (
          <div style={{ display: "contents" }}>
            {i() > 0 && (
              <div
                class={isVertical() ? "terminal-split-divider-vertical" : "terminal-split-divider"}
                onMouseDown={(e) => handleDividerMouseDown(e, i() - 1)}
              />
            )}
            <div
              class={["terminal-split-pane", { "active-pane": pane().paneId === props.group.activePaneId }]}
              style={{ flex: flexValues()[i()] ?? 1 }}
              onMouseDown={() => repositoryStore.getState().setActivePaneInGroup(props.group.groupId, pane().paneId)}
            >
              <TerminalInstance
                paneId={pane().paneId}
                isActive={props.isFocused && pane().paneId === props.group.activePaneId}
              />
            </div>
          </div>
        )}
      </For>
    </div>
  );
};
