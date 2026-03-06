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
      const next = Array.from({ length: count }, (_, i) => prev[i] ?? 1);
      return next;
    });
  };

  if (flexValues.length !== group.panes.length) {
    syncFlexCount(group.panes.length);
  }

  const handleDividerMouseDown = useCallback(
    (e: React.MouseEvent, dividerIndex: number) => {
      e.preventDefault();
      const container = containerRef.current;
      if (!container) return;

      const paneEls = container.querySelectorAll<HTMLElement>(".terminal-split-pane");
      const leftPane = paneEls[dividerIndex];
      const rightPane = paneEls[dividerIndex + 1];
      if (!leftPane || !rightPane) return;

      const startX = e.clientX;
      const leftStartWidth = leftPane.getBoundingClientRect().width;
      const rightStartWidth = rightPane.getBoundingClientRect().width;
      const totalWidth = leftStartWidth + rightStartWidth;

      const handleMouseMove = (moveEvent: MouseEvent) => {
        const delta = moveEvent.clientX - startX;
        const newLeftWidth = Math.max(80, leftStartWidth + delta);
        const newRightWidth = Math.max(80, rightStartWidth - delta);
        const clampedLeft = totalWidth - newRightWidth < 80 ? 80 : newLeftWidth;
        const clampedRight = totalWidth - clampedLeft;

        setFlexValues((prev) => {
          const next = [...prev];
          next[dividerIndex] = clampedLeft / totalWidth;
          next[dividerIndex + 1] = clampedRight / totalWidth;
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
    [],
  );

  return (
    <div
      ref={containerRef}
      className="terminal-group-view"
      style={{ display: isActive ? "flex" : "none" }}
    >
      {group.panes.map((pane, i) => (
        <div key={pane.paneId} style={{ display: "contents" }}>
          {i > 0 && (
            <div
              className="terminal-split-divider"
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
