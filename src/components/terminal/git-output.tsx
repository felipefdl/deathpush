import { createEffect, createSignal, For, onSettled } from "solid-js";
import { listen } from "@tauri-apps/api/event";

type GitCommandEntry = {
  command: string;
  duration_ms: number;
  timestamp: string;
};

export const GitOutput = () => {
  const [entries, setEntries] = createSignal<GitCommandEntry[]>([]);
  let containerEl: HTMLDivElement | undefined;

  onSettled(() => {
    const unlisten = listen<GitCommandEntry>("git:command", (event) => {
      setEntries((prev) => [...prev, event.payload]);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  });

  createEffect(
    () => entries(),
    () => {
      if (containerEl) {
        containerEl.scrollTop = containerEl.scrollHeight;
      }
    }
  );

  return (
    <div class="git-output" ref={(el) => (containerEl = el)}>
      {entries().length === 0 ? (
        <div class="git-output-empty">No git commands recorded yet.</div>
      ) : (
        <For each={entries()} keyed={false}>
          {(entry) => (
            <div class="git-output-line">
              <span class="git-output-timestamp">{entry().timestamp}</span>
              <span class="git-output-level">[info]</span>
              <span class="git-output-arrow">&gt;</span>
              <span class="git-output-command">{entry().command}</span>
              <span class="git-output-duration">[{entry().duration_ms}ms]</span>
            </div>
          )}
        </For>
      )}
    </div>
  );
};
