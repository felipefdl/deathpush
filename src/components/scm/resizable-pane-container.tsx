import { createEffect, createMemo, createSignal, For } from "solid-js";
import type { JSX } from "@solidjs/web";
import { useResizeObserver } from "../../hooks/use-resize-observer";
import { layoutStore } from "../../stores/layout-store";
import { useStore } from "../../lib/use-store";

type PaneDefinition = {
  id: string;
  defaultRatio?: number;
  defaultCollapsed?: boolean;
  header: (collapsed: boolean, onToggle: () => void) => JSX.Element;
  body: () => JSX.Element;
};

type PaneRatio = {
  heightRatio: number;
};

const MIN_PANE_HEIGHT = 60;
const HEADER_HEIGHT = 22;
const DIVIDER_HEIGHT = 4;

export type { PaneDefinition };

export const ResizablePaneContainer = (props: { panes: PaneDefinition[] }) => {
  const { ref: containerRef, height: containerHeight } = useResizeObserver();
  const collapsedPanes = useStore(layoutStore, (s) => s.collapsedPanes);
  const { togglePaneCollapsed } = layoutStore.getState();
  const [paneRatios, setPaneRatios] = createSignal<Record<string, PaneRatio>>({});
  let dragRef: {
    paneAbove: string;
    paneBelow: string;
    startY: number;
    startRatioAbove: number;
    startRatioBelow: number;
  } | null = null;

  const seenPanes = new Set<string>();
  const isCollapsed = (id: string) => collapsedPanes().includes(id);
  const getRatio = (id: string) => paneRatios()[id]?.heightRatio ?? 1;

  createEffect(
    () => [props.panes, collapsedPanes()] as const,
    ([panes, collapsed]) => {
      for (const pane of panes) {
        if (pane.defaultCollapsed && !seenPanes.has(pane.id) && !collapsed.includes(pane.id)) {
          togglePaneCollapsed(pane.id);
        }
        seenPanes.add(pane.id);
      }
    }
  );

  createEffect(
    () => [props.panes, collapsedPanes()] as const,
    ([panes]) => {
      setPaneRatios((prev) => {
        const paneIds = new Set(panes.map((p) => p.id));
        const next: Record<string, PaneRatio> = {};
        let hasNew = false;

        for (const pane of panes) {
          if (prev[pane.id]) {
            next[pane.id] = prev[pane.id];
          } else {
            next[pane.id] = { heightRatio: pane.defaultRatio ?? 1 };
            hasNew = true;
          }
        }

        const hadOld = Object.keys(prev).some((id) => !paneIds.has(id));
        if (!hasNew && !hadOld) return prev;

        const expandedIds = panes.filter((p) => !isCollapsed(p.id)).map((p) => p.id);
        if (expandedIds.length > 0) {
          const totalRatio = expandedIds.reduce((sum, id) => sum + (next[id]?.heightRatio ?? 1), 0);
          if (totalRatio > 0) {
            for (const id of expandedIds) {
              next[id] = { heightRatio: (next[id]?.heightRatio ?? 1) / totalRatio };
            }
          } else {
            const equal = 1 / expandedIds.length;
            for (const id of expandedIds) {
              next[id] = { heightRatio: equal };
            }
          }
        }

        return next;
      });
    }
  );

  const togglePane = (id: string) => {
    const willBeCollapsed = !isCollapsed(id);
    togglePaneCollapsed(id);

    setPaneRatios((prev) => {
      const next = { ...prev };
      const expandedIds = props.panes
        .filter((p) => (p.id === id ? !willBeCollapsed : !isCollapsed(p.id)))
        .map((p) => p.id);

      if (expandedIds.length > 0) {
        const totalRatio = expandedIds.reduce((sum, k) => sum + (next[k]?.heightRatio ?? 1), 0);
        if (totalRatio > 0) {
          for (const k of expandedIds) {
            next[k] = { heightRatio: (next[k]?.heightRatio ?? 1) / totalRatio };
          }
        } else {
          const equal = 1 / expandedIds.length;
          for (const k of expandedIds) {
            next[k] = { heightRatio: equal };
          }
        }
      }

      return next;
    });
  };

  const expanded = createMemo(() => props.panes.filter((p) => !isCollapsed(p.id)));
  const collapsed = createMemo(() => props.panes.filter((p) => isCollapsed(p.id)));

  const availableBodyHeight = createMemo(() => {
    const collapsedAreaHeight = collapsed().length * HEADER_HEIGHT;
    const dividersHeight = Math.max(0, expanded().length - 1) * DIVIDER_HEIGHT;
    const expandedHeadersHeight = expanded().length * HEADER_HEIGHT;
    return Math.max(0, containerHeight() - collapsedAreaHeight - dividersHeight - expandedHeadersHeight);
  });

  const handleDividerMouseDown = (e: MouseEvent, aboveId: string, belowId: string) => {
    e.preventDefault();
    const aboveRatio = getRatio(aboveId);
    const belowRatio = getRatio(belowId);
    const bodyHeight = availableBodyHeight();

    dragRef = {
      paneAbove: aboveId,
      paneBelow: belowId,
      startY: e.clientY,
      startRatioAbove: aboveRatio,
      startRatioBelow: belowRatio,
    };

    const totalRatio = aboveRatio + belowRatio;
    const minRatio = bodyHeight > 0 ? MIN_PANE_HEIGHT / bodyHeight : 0;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const drag = dragRef;
      if (!drag || bodyHeight <= 0) return;

      const deltaY = moveEvent.clientY - drag.startY;
      const deltaRatio = deltaY / bodyHeight;

      let newAbove = drag.startRatioAbove + deltaRatio;
      let newBelow = drag.startRatioBelow - deltaRatio;

      if (newAbove < minRatio) {
        newAbove = minRatio;
        newBelow = totalRatio - minRatio;
      }
      if (newBelow < minRatio) {
        newBelow = minRatio;
        newAbove = totalRatio - minRatio;
      }

      setPaneRatios((prev) => ({
        ...prev,
        [drag.paneAbove]: { heightRatio: newAbove },
        [drag.paneBelow]: { heightRatio: newBelow },
      }));
    };

    const handleMouseUp = () => {
      dragRef = null;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  };

  return (
    <div class="resizable-pane-container" ref={containerRef}>
      {props.panes.length > 0 && (
        <>
          <div class="resizable-pane-expanded-area">
            <For each={expanded()} keyed={(pane) => pane.id}>
              {(pane, i) => (
                <div class="resizable-pane-wrapper" style={{ flex: `${getRatio(pane().id)} 1 0px` }}>
                  <div class="resizable-pane">
                    {pane().header(false, () => togglePane(pane().id))}
                    <div class="resizable-pane-body">{pane().body()}</div>
                  </div>
                  {i() < expanded().length - 1 && (
                    <div
                      class="pane-divider"
                      onMouseDown={(e) => handleDividerMouseDown(e, expanded()[i()].id, expanded()[i() + 1].id)}
                    />
                  )}
                </div>
              )}
            </For>
          </div>
          {collapsed().length > 0 && (
            <div class="resizable-pane-collapsed-area">
              <For each={collapsed()} keyed={(pane) => pane.id}>
                {(pane) => <div>{pane().header(true, () => togglePane(pane().id))}</div>}
              </For>
            </div>
          )}
        </>
      )}
    </div>
  );
};
