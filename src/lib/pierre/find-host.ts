const FIND_SELECTOR = "[data-content] [data-line], [data-column-content]";
const HIGHLIGHT_ALL = "deathpush-find";
const HIGHLIGHT_CURRENT = "deathpush-find-current";

export type PierreFindScanRoot = {
  querySelectorAll: (selectors: string) => ArrayLike<Element>;
};

export type PierreFindHost = {
  open: () => void;
  close: () => void;
  next: (dir?: 1 | -1) => void;
  isOpen: () => boolean;
  dispose: () => void;
};

type HostEntry = PierreFindHost & { wrapper: HTMLElement };

const hosts = new Set<HostEntry>();
let listenerCount = 0;
let highlightStyle: HTMLStyleElement | null = null;

type HighlightSet = { set: (name: string, highlight: Highlight) => void; delete: (name: string) => void };

const highlightCtor = (): (new (...ranges: Range[]) => Highlight) | undefined =>
  (globalThis as { Highlight?: new (...ranges: Range[]) => Highlight }).Highlight;

const cssHighlights = (): HighlightSet | null => {
  const api = (globalThis as { CSS?: { highlights?: HighlightSet } }).CSS?.highlights;
  return highlightCtor() && api ? api : null;
};

const ensureHighlightStyle = (): void => {
  if (highlightStyle || typeof document === "undefined") return;
  highlightStyle = document.createElement("style");
  const matchBg = "var(--vscode-editor-findMatchHighlightBackground, rgba(234, 92, 0, 0.33))";
  const currentBg = "var(--vscode-editor-findMatchBackground, rgba(234, 92, 0, 0.66))";
  highlightStyle.textContent = [
    `::highlight(${HIGHLIGHT_ALL}) { background-color: ${matchBg}; }`,
    `::highlight(${HIGHLIGHT_CURRENT}) { background-color: ${currentBg}; }`,
  ].join("\n");
  document.head.append(highlightStyle);
};

const locate = (nodes: Text[], ends: number[], offset: number): { node: Text; offset: number } => {
  let lo = 0;
  let hi = ends.length - 1;
  while (lo < hi) {
    const mid = (lo + hi) >> 1;
    if (ends[mid] >= offset) hi = mid;
    else lo = mid + 1;
  }
  return { node: nodes[lo], offset: offset - (lo === 0 ? 0 : ends[lo - 1]) };
};

export const scanPierreFind = (root: PierreFindScanRoot, query: string): Range[] => {
  const needle = query.trim().toLowerCase();
  if (!needle) return [];

  const ranges: Range[] = [];
  for (const col of Array.from(root.querySelectorAll(FIND_SELECTOR))) {
    if (!(col instanceof HTMLElement)) continue;
    const text = col.textContent;
    if (!text) continue;

    const hay = text.toLowerCase();
    let at = hay.indexOf(needle);
    if (at === -1) continue;

    const nodes: Text[] = [];
    const ends: number[] = [];
    const walker = document.createTreeWalker(col, NodeFilter.SHOW_TEXT);
    let node = walker.nextNode();
    let pos = 0;
    while (node) {
      if (node instanceof Text) {
        pos += node.data.length;
        nodes.push(node);
        ends.push(pos);
      }
      node = walker.nextNode();
    }
    if (nodes.length === 0) continue;

    while (at !== -1) {
      const start = locate(nodes, ends, at);
      const end = locate(nodes, ends, at + needle.length);
      const range = document.createRange();
      range.setStart(start.node, start.offset);
      range.setEnd(end.node, end.offset);
      ranges.push(range);
      at = hay.indexOf(needle, at + needle.length);
    }
  }
  return ranges;
};

export const isPierreEditorFocused = (node: Element | null = document.activeElement): boolean => {
  let current: Element | null = node;
  while (current) {
    if (current instanceof HTMLElement && current.isContentEditable) return true;
    const inner = current.shadowRoot?.activeElement;
    if (!inner) return false;
    current = inner;
  }
  return false;
};

export const isPierreFindHostOpen = (): boolean => [...hosts].some((host) => host.isOpen());

const hostForFocus = (): HostEntry | undefined => {
  const active = document.activeElement;
  if (active) {
    for (const host of hosts) {
      if (host.wrapper.contains(active)) return host;
    }
  }
  return [...hosts].find((host) => host.wrapper.isConnected);
};

const clearHighlights = (): void => {
  const api = cssHighlights();
  if (!api) return;
  api.delete(HIGHLIGHT_ALL);
  api.delete(HIGHLIGHT_CURRENT);
};

const applyHighlights = (ranges: Range[], currentIndex: number): boolean => {
  const api = cssHighlights();
  if (!api) return false;
  clearHighlights();
  const HighlightAPI = highlightCtor();
  if (!HighlightAPI) return false;
  const active = ranges[currentIndex];
  if (active) api.set(HIGHLIGHT_CURRENT, new HighlightAPI(active));
  const rest = ranges.filter((_, index) => index !== currentIndex);
  if (rest.length > 0) api.set(HIGHLIGHT_ALL, new HighlightAPI(...rest));
  return true;
};

const paintOverlay = (overlay: HTMLElement, wrapper: HTMLElement, ranges: Range[], currentIndex: number): void => {
  overlay.replaceChildren();
  const base = wrapper.getBoundingClientRect();
  const frag = document.createDocumentFragment();
  for (const [index, range] of ranges.entries()) {
    const active = index === currentIndex;
    for (const rect of range.getClientRects()) {
      if (!rect.width || !rect.height) continue;
      const mark = document.createElement("div");
      mark.style.cssText = [
        "position:absolute",
        `left:${Math.round(rect.left - base.left)}px`,
        `top:${Math.round(rect.top - base.top)}px`,
        `width:${Math.round(rect.width)}px`,
        `height:${Math.round(rect.height)}px`,
        "border-radius:2px",
        `background:${
          active
            ? "var(--vscode-editor-findMatchBackground, rgba(234, 92, 0, 0.66))"
            : "var(--vscode-editor-findMatchHighlightBackground, rgba(234, 92, 0, 0.33))"
        }`,
        "pointer-events:none",
      ].join(";");
      frag.append(mark);
    }
  }
  overlay.append(frag);
};

const button = (label: string, title: string, onClick: () => void): HTMLButtonElement => {
  const el = document.createElement("button");
  el.type = "button";
  el.textContent = label;
  el.title = title;
  el.style.cssText = [
    "display:inline-flex",
    "align-items:center",
    "justify-content:center",
    "height:22px",
    "min-width:22px",
    "padding:0 6px",
    "border:1px solid var(--vscode-button-border, var(--vscode-panel-border))",
    "background:var(--vscode-button-secondaryBackground)",
    "color:var(--vscode-button-secondaryForeground)",
    "cursor:pointer",
    "border-radius:var(--radius-sm)",
    "font:12px/1 var(--vscode-font-family)",
  ].join(";");
  el.addEventListener("click", (event) => {
    event.preventDefault();
    event.stopPropagation();
    onClick();
  });
  return el;
};

const onWindowKeyDown = (event: KeyboardEvent): void => {
  if (event.key === "Escape") {
    const open = [...hosts].find((host) => host.isOpen());
    if (!open) return;
    event.preventDefault();
    open.close();
    return;
  }

  const isMod = event.metaKey || event.ctrlKey;
  if (!isMod || event.key.toLowerCase() !== "f") return;
  if (isPierreEditorFocused()) return;
  const host = hostForFocus();
  if (!host) return;
  event.preventDefault();
  host.open();
};

const retainListener = (): void => {
  if (listenerCount === 0) window.addEventListener("keydown", onWindowKeyDown, true);
  listenerCount += 1;
};

const releaseListener = (): void => {
  listenerCount -= 1;
  if (listenerCount === 0) window.removeEventListener("keydown", onWindowKeyDown, true);
};

export const createPierreFindHost = (opts: {
  getRoot: () => PierreFindScanRoot | null | undefined;
  wrapper: HTMLElement;
}): PierreFindHost => {
  const { getRoot, wrapper } = opts;
  let open = false;
  let query = "";
  let index = 0;
  let hits: Range[] = [];

  const bar = document.createElement("div");
  bar.style.cssText = [
    "display:none",
    "position:absolute",
    "top:8px",
    "right:8px",
    "z-index:5",
    "align-items:center",
    "gap:4px",
    "height:28px",
    "padding:0 6px",
    "background:var(--vscode-editorWidget-background, var(--vscode-editor-background))",
    "color:var(--vscode-editorWidget-foreground, var(--vscode-editor-foreground))",
    "border:1px solid var(--vscode-editorWidget-border, var(--vscode-panel-border))",
    "border-radius:var(--radius-sm)",
    "box-shadow:0 2px 8px rgba(0,0,0,0.24)",
  ].join(";");

  const input = document.createElement("input");
  input.type = "search";
  input.placeholder = "Find";
  input.style.cssText = [
    "height:22px",
    "width:160px",
    "padding:0 6px",
    "border:1px solid var(--vscode-input-border, var(--vscode-panel-border))",
    "background:var(--vscode-input-background)",
    "color:var(--vscode-input-foreground)",
    "font:12px/1 var(--vscode-font-family)",
    "outline:none",
  ].join(";");

  const count = document.createElement("span");
  count.style.cssText = "min-width:36px;font:12px/1 var(--vscode-font-family);opacity:0.8;text-align:right;";

  const overlay = document.createElement("div");
  overlay.style.cssText = "position:absolute;inset:0;pointer-events:none;z-index:4;";

  const syncCount = (): void => {
    count.textContent = hits.length === 0 ? "0/0" : `${index + 1}/${hits.length}`;
  };

  const paint = (): void => {
    if (cssHighlights() && applyHighlights(hits, index)) {
      overlay.replaceChildren();
      return;
    }
    clearHighlights();
    paintOverlay(overlay, wrapper, hits, index);
  };

  const apply = (reset: boolean): void => {
    const root = getRoot();
    hits = root ? scanPierreFind(root, query) : [];
    index = hits.length === 0 ? 0 : reset ? 0 : Math.min(index, hits.length - 1);
    syncCount();
    paint();
    const active = hits[index];
    if (!active) return;
    const node = active.startContainer;
    const el = node instanceof Element ? node : node.parentElement;
    el?.scrollIntoView({ block: "center", inline: "nearest" });
  };

  const close = (): void => {
    open = false;
    query = "";
    index = 0;
    hits = [];
    input.value = "";
    bar.style.display = "none";
    overlay.replaceChildren();
    clearHighlights();
    syncCount();
  };

  const next = (dir: 1 | -1 = 1): void => {
    if (!open || hits.length === 0) return;
    index = (index + dir + hits.length) % hits.length;
    syncCount();
    paint();
    const active = hits[index];
    const node = active?.startContainer;
    const el = node instanceof Element ? node : node?.parentElement;
    el?.scrollIntoView({ block: "center", inline: "nearest" });
  };

  const openHost = (): void => {
    open = true;
    if (getComputedStyle(wrapper).position === "static") wrapper.style.position = "relative";
    bar.style.display = "inline-flex";
    apply(false);
    input.focus();
    input.select();
  };

  input.addEventListener("input", () => {
    query = input.value;
    apply(true);
  });
  input.addEventListener("keydown", (event) => {
    if (event.key === "Escape") {
      event.preventDefault();
      close();
      return;
    }
    if (event.key !== "Enter") return;
    event.preventDefault();
    next(event.shiftKey ? -1 : 1);
  });

  bar.append(
    input,
    count,
    button("<", "Previous match", () => next(-1)),
    button(">", "Next match", () => next(1)),
    button("x", "Close", close)
  );
  wrapper.append(bar, overlay);
  syncCount();

  const host: HostEntry = {
    wrapper,
    open: openHost,
    close,
    next,
    isOpen: () => open,
    dispose: () => {
      close();
      bar.remove();
      overlay.remove();
      hosts.delete(host);
      releaseListener();
    },
  };

  ensureHighlightStyle();
  hosts.add(host);
  retainListener();
  return host;
};
