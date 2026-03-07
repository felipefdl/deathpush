import { useCallback, useRef, useState } from "react";
import type { TerminalGroup } from "../../stores/repository-store";
import { useRepositoryStore } from "../../stores/repository-store";
import { TerminalInstance } from "./terminal-instance";

interface TerminalGroupViewProps {
  group: TerminalGroup;
  isActive: boolean;
}

export const TerminalGroupView = ({ group, isActive }: TerminalGroupViewProps) => {
  const setActivePaneInGroup = useRepositoryStore((s) => s.setActivePaneInGroup);
  const containerRef = useRef<HTMLDivElement>(null);
  const [flexValues, setFlexValues] = useState<number[]>(() => group.panes.map(() => 1));

  const syncFlexCount = (count: number) => {
    setFlexValues((prev) => {
      if (prev.length === count) return prev;
      if (count < prev.length) {
        return Array.from({ length: count }, () => 1);
      }
      return Array.from({ length: count }, (_, i) => prev[i] ?? 1);
    });
  };

  if (flexValues.length !== group.panes.length) {
    syncFlexCount(group.panes.length);
  }

  const isVertical = group.splitDirection === "vertical";

  const handleDividerMouseDown = useCallback(
    (e: React.MouseEvent, dividerIndex: number) => {
      e.preventDefault();
      const container = containerRef.current;
      if (!container) return;

      const paneEls = container.querySelectorAll<HTMLElement>(".terminal-split-pane");
      const firstPane = paneEls[dividerIndex];
      const secondPane = paneEls[dividerIndex + 1];
      if (!firstPane || !secondPane) return;

      const vertical = group.splitDirection === "vertical";
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
    },
    [group.splitDirection],
  );

  return (
    <div
      ref={containerRef}
      className="terminal-group-view"
      style={{ display: isActive ? "flex" : "none", flexDirection: isVertical ? "column" : "row" }}
    >
      {group.panes.map((pane, i) => (
        <div key={pane.paneId} style={{ display: "contents" }}>
          {i > 0 && (
            <div
              className={isVertical ? "terminal-split-divider-vertical" : "terminal-split-divider"}
              onMouseDown={(e) => handleDividerMouseDown(e, i - 1)}
            />
          )}
          <div
            className={`terminal-split-pane ${pane.paneId === group.activePaneId ? "active-pane" : ""}`}
            style={{ flex: flexValues[i] ?? 1 }}
            onMouseDown={() => setActivePaneInGroup(group.groupId, pane.paneId)}
          >
            <TerminalInstance
              paneId={pane.paneId}
              isActive={isActive && pane.paneId === group.activePaneId}
            />
          </div>
        </div>
      ))}
    </div>
  );
};
