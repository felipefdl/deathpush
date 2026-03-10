import { type ReactNode, useState, useEffect, useCallback, useRef } from "react";
import { useResizeObserver } from "../../hooks/use-resize-observer";
import { useLayoutStore } from "../../stores/layout-store";

interface PaneDefinition {
  id: string;
  defaultRatio?: number;
  header: (collapsed: boolean, onToggle: () => void) => ReactNode;
  body: () => ReactNode;
}

interface PaneRatio {
  heightRatio: number;
}

const MIN_PANE_HEIGHT = 60;
const HEADER_HEIGHT = 22;
const DIVIDER_HEIGHT = 4;

export type { PaneDefinition };

export const ResizablePaneContainer = ({ panes }: { panes: PaneDefinition[] }) => {
  const { ref: containerRef, height: containerHeight } = useResizeObserver();
  const collapsedPanes = useLayoutStore((s) => s.collapsedPanes);
  const togglePaneCollapsed = useLayoutStore((s) => s.togglePaneCollapsed);
  const [paneRatios, setPaneRatios] = useState<Record<string, PaneRatio>>({});
  const dragRef = useRef<{
    paneAbove: string;
    paneBelow: string;
    startY: number;
    startRatioAbove: number;
    startRatioBelow: number;
  } | null>(null);

  const isCollapsed = (id: string) => collapsedPanes.includes(id);
  const getRatio = (id: string) => paneRatios[id]?.heightRatio ?? 1;

  // Sync pane ratios when panes change
  useEffect(() => {
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
  }, [panes, collapsedPanes]);

  const togglePane = useCallback((id: string) => {
    togglePaneCollapsed(id);

    setPaneRatios((prev) => {
      const next = { ...prev };
      const willBeCollapsed = !isCollapsed(id);
      const expandedIds = panes
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
  }, [panes, togglePaneCollapsed, collapsedPanes]);

  const expanded = panes.filter((p) => !isCollapsed(p.id));
  const collapsed = panes.filter((p) => isCollapsed(p.id));

  // Compute available body height for drag min-ratio clamping
  const collapsedAreaHeight = collapsed.length * HEADER_HEIGHT;
  const dividersHeight = Math.max(0, expanded.length - 1) * DIVIDER_HEIGHT;
  const expandedHeadersHeight = expanded.length * HEADER_HEIGHT;
  const availableBodyHeight = Math.max(0, containerHeight - collapsedAreaHeight - dividersHeight - expandedHeadersHeight);

  const handleDividerMouseDown = useCallback((e: React.MouseEvent, aboveId: string, belowId: string) => {
    e.preventDefault();
    const aboveRatio = getRatio(aboveId);
    const belowRatio = getRatio(belowId);

    dragRef.current = {
      paneAbove: aboveId,
      paneBelow: belowId,
      startY: e.clientY,
      startRatioAbove: aboveRatio,
      startRatioBelow: belowRatio,
    };

    const totalRatio = aboveRatio + belowRatio;
    const minRatio = availableBodyHeight > 0 ? MIN_PANE_HEIGHT / availableBodyHeight : 0;

    const handleMouseMove = (moveEvent: MouseEvent) => {
      const drag = dragRef.current;
      if (!drag || availableBodyHeight <= 0) return;

      const deltaY = moveEvent.clientY - drag.startY;
      const deltaRatio = deltaY / availableBodyHeight;

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
      dragRef.current = null;
      document.removeEventListener("mousemove", handleMouseMove);
      document.removeEventListener("mouseup", handleMouseUp);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };

    document.body.style.cursor = "ns-resize";
    document.body.style.userSelect = "none";
    document.addEventListener("mousemove", handleMouseMove);
    document.addEventListener("mouseup", handleMouseUp);
  }, [paneRatios, availableBodyHeight]);

  return (
    <div className="resizable-pane-container" ref={containerRef}>
      {panes.length > 0 && (
        <>
          <div className="resizable-pane-expanded-area">
            {expanded.map((pane, i) => {
              const ratio = getRatio(pane.id);
              return (
                <div key={pane.id} className="resizable-pane-wrapper" style={{ flex: `${ratio} 1 0px` }}>
                  <div className="resizable-pane">
                    {pane.header(false, () => togglePane(pane.id))}
                    <div className="resizable-pane-body">
                      {pane.body()}
                    </div>
                  </div>
                  {i < expanded.length - 1 && (
                    <div
                      className="pane-divider"
                      onMouseDown={(e) => handleDividerMouseDown(e, expanded[i].id, expanded[i + 1].id)}
                    />
                  )}
                </div>
              );
            })}
          </div>
          {collapsed.length > 0 && (
            <div className="resizable-pane-collapsed-area">
              {collapsed.map((pane) => (
                <div key={pane.id}>
                  {pane.header(true, () => togglePane(pane.id))}
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
};
