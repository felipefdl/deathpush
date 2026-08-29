import { createSignal, onSettled } from "solid-js";
import type { SearchAddon } from "@xterm/addon-search";

type TerminalSearchBarProps = {
  searchAddon: SearchAddon;
  onClose: () => void;
};

export const TerminalSearchBar = (props: TerminalSearchBarProps) => {
  const [query, setQuery] = createSignal("");
  const [resultIndex, setResultIndex] = createSignal(-1);
  const [resultCount, setResultCount] = createSignal(0);
  let inputEl: HTMLInputElement | undefined;

  onSettled(() => {
    inputEl?.focus();
    const disposable = props.searchAddon.onDidChangeResults?.((event) => {
      setResultIndex(event.resultIndex);
      setResultCount(event.resultCount);
    });
    return () => disposable?.dispose();
  });

  const handleInput = (e: InputEvent & { currentTarget: HTMLInputElement }) => {
    const value = e.currentTarget.value;
    setQuery(value);
    if (value) {
      props.searchAddon.findNext(value, { incremental: true });
    } else {
      props.searchAddon.clearDecorations();
      setResultIndex(-1);
      setResultCount(0);
    }
  };

  const handleKeyDown = (e: KeyboardEvent) => {
    if (e.key === "Escape") {
      props.searchAddon.clearDecorations();
      props.onClose();
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (e.shiftKey) {
        props.searchAddon.findPrevious(query());
      } else {
        props.searchAddon.findNext(query());
      }
    }
  };

  const countLabel = () => {
    const q = query();
    const count = resultCount();
    return q && count >= 0 ? `${resultIndex() + 1}/${count}` : "";
  };

  return (
    <div class="terminal-search-bar">
      <input
        ref={(el) => (inputEl = el)}
        class="terminal-search-input"
        type="text"
        value={query()}
        placeholder="Find"
        spellcheck={false}
        onInput={handleInput}
        onKeyDown={handleKeyDown}
      />
      {countLabel() && <span class="terminal-search-count">{countLabel()}</span>}
      <button
        class="terminal-search-btn"
        onClick={() => props.searchAddon.findPrevious(query())}
        title="Previous Match (Shift+Enter)"
      >
        <span class="codicon codicon-chevron-up" />
      </button>
      <button
        class="terminal-search-btn"
        onClick={() => props.searchAddon.findNext(query())}
        title="Next Match (Enter)"
      >
        <span class="codicon codicon-chevron-down" />
      </button>
      <button
        class="terminal-search-btn"
        onClick={() => {
          props.searchAddon.clearDecorations();
          props.onClose();
        }}
        title="Close (Escape)"
      >
        <span class="codicon codicon-close" />
      </button>
    </div>
  );
};
