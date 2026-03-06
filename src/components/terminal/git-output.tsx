import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";

interface GitCommandEntry {
  command: string;
  duration_ms: number;
  timestamp: string;
}

export const GitOutput = () => {
  const [entries, setEntries] = useState<GitCommandEntry[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const unlisten = listen<GitCommandEntry>("git:command", (event) => {
      setEntries((prev) => [...prev, event.payload]);
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    if (containerRef.current) {
      containerRef.current.scrollTop = containerRef.current.scrollHeight;
    }
  }, [entries]);

  return (
    <div className="git-output" ref={containerRef}>
      {entries.length === 0 ? (
        <div className="git-output-empty">No git commands recorded yet.</div>
      ) : (
        entries.map((entry, i) => (
          <div key={i} className="git-output-line">
            <span className="git-output-timestamp">{entry.timestamp}</span>
            <span className="git-output-level">[info]</span>
            <span className="git-output-arrow">&gt;</span>
            <span className="git-output-command">{entry.command}</span>
            <span className="git-output-duration">[{entry.duration_ms}ms]</span>
          </div>
        ))
      )}
    </div>
  );
};
