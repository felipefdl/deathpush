import { cleanup, render } from "@solidjs/testing-library";
import { createSignal, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import type { SaveSession } from "../../lib/pierre/save-session";

const mocks = vi.hoisted(() => ({
  editorConstructed: vi.fn(),
  fileConstructed: vi.fn(),
  fileRendered: vi.fn(),
  virtualizerConstructed: vi.fn(),
  mountingDuringRender: vi.fn(),
}));

vi.mock("@pierre/diffs", () => ({
  DEFAULT_VIRTUAL_FILE_METRICS: {
    hunkLineCount: 20,
    lineHeight: 20,
    diffHeaderHeight: 0,
    spacing: 8,
  },
  getFiletypeFromFileName: () => "text",
  areLanguagesAttached: () => true,
  getSharedHighlighter: vi.fn(async () => ({})),
  Virtualizer: class {
    constructor() {
      mocks.virtualizerConstructed();
    }
    setup() {}
    cleanUp() {}
  },
  VirtualizedFile: class {
    options: { onPostRender?: (node: HTMLElement, instance: unknown, phase: "mount") => void };
    constructor(options: { onPostRender?: (node: HTMLElement, instance: unknown, phase: "mount") => void }) {
      this.options = options;
      mocks.fileConstructed();
    }
    render(props: { containerWrapper: HTMLElement }) {
      mocks.mountingDuringRender(props.containerWrapper.hasAttribute("data-pierre-mounting"));
      mocks.fileRendered(props);
      this.options.onPostRender?.(document.createElement("div"), this, "mount");
    }
    setOptions(options: { onPostRender?: (node: HTMLElement, instance: unknown, phase: "mount") => void }) {
      this.options = options;
    }
    setMetrics() {}
    cleanUp() {}
  },
}));

vi.mock("@pierre/diffs/edit", () => ({
  Editor: class {
    constructor(options: unknown) {
      mocks.editorConstructed(options);
    }
    edit() {
      return () => undefined;
    }
    getState() {
      return {};
    }
    focus() {}
    cleanUp() {}
  },
}));

vi.mock("../../lib/pierre/worker", () => ({
  getPierreWorkerPool: () => ({}),
}));

vi.mock("../../lib/pierre/flush-registry", () => ({
  registerFlusher: () => () => undefined,
  trackPendingFlush: (_path: string, promise: Promise<void>) => promise,
}));

vi.mock("../../lib/tauri-commands", () => ({
  writeFile: vi.fn(async () => undefined),
}));

import { PierreFile } from "./pierre-file";

type FileInput = {
  path: string;
  contents: string;
  cacheKey: string;
  session: SaveSession;
};

const input = (path: string): FileInput => ({
  path,
  contents: `contents:${path}`,
  cacheKey: `cache:${path}`,
  session: { path, diskSha: `sha:${path}`, pendingSha: null, cacheGeneration: 0 },
});

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("PierreFile navigation", () => {
  it("reuses one editor runtime while switching files", () => {
    let selectFile!: (next: FileInput) => void;
    const Harness = () => {
      const [file, setFile] = createSignal(input("NOTICE"));
      selectFile = setFile;
      return (
        <PierreFile
          path={file().path}
          contents={file().contents}
          cacheKey={file().cacheKey}
          revealLine={null}
          session={file().session}
        />
      );
    };

    render(() => <Harness />);
    flush();
    selectFile(input("LICENSE"));
    flush();

    expect(mocks.virtualizerConstructed).toHaveBeenCalledTimes(1);
    expect(mocks.fileConstructed).toHaveBeenCalledTimes(1);
    expect(mocks.editorConstructed).toHaveBeenCalledTimes(1);
    expect(mocks.editorConstructed).toHaveBeenCalledWith(expect.objectContaining({ persistState: true }));
    expect(mocks.fileRendered).toHaveBeenCalledTimes(2);
    expect(mocks.mountingDuringRender).toHaveBeenCalledTimes(2);
    expect(mocks.mountingDuringRender).toHaveBeenCalledWith(true);
  });

  it("keeps a visible thumb synchronized with the scroll position", () => {
    const view = render(() => {
      const file = input("README.md");
      return (
        <PierreFile
          path={file.path}
          contents={file.contents}
          cacheKey={file.cacheKey}
          revealLine={null}
          session={file.session}
        />
      );
    });
    flush();

    const host = view.container.querySelector<HTMLElement>(".pierre-file-host");
    const track = view.container.querySelector<HTMLElement>(".pierre-file-scrollbar");
    const thumb = view.container.querySelector<HTMLElement>(".pierre-file-scrollbar-thumb");
    expect(host).not.toBeNull();
    expect(track).not.toBeNull();
    expect(thumb).not.toBeNull();
    if (!host || !track || !thumb) return;

    expect(track.hidden).toBe(false);
    expect(thumb.hidden).toBe(true);

    Object.defineProperties(host, {
      clientHeight: { configurable: true, value: 500 },
      scrollHeight: { configurable: true, value: 1500 },
      scrollTop: { configurable: true, value: 500, writable: true },
    });
    Object.defineProperty(track, "clientHeight", { configurable: true, value: 500 });
    host.dispatchEvent(new Event("scroll"));

    expect(thumb.hidden).toBe(false);
    expect(Number.parseFloat(thumb.style.height)).toBeCloseTo(166.67, 1);
    expect(Number.parseFloat(thumb.style.transform.slice("translateY(".length))).toBeCloseTo(166.67, 1);
  });

  it("keeps the horizontal thumb at the viewport bottom", () => {
    const view = render(() => {
      const file = input("README.md");
      return (
        <PierreFile
          path={file.path}
          contents={file.contents}
          cacheKey={file.cacheKey}
          revealLine={null}
          session={file.session}
        />
      );
    });
    flush();

    const host = view.container.querySelector<HTMLElement>(".pierre-file-host");
    const content = view.container.querySelector<HTMLElement>(".pierre-file-content");
    const track = view.container.querySelector<HTMLElement>(".pierre-file-scrollbar-horizontal");
    const thumb = view.container.querySelector<HTMLElement>(".pierre-file-scrollbar-horizontal-thumb");
    expect(host).not.toBeNull();
    expect(content).not.toBeNull();
    expect(track).not.toBeNull();
    expect(thumb).not.toBeNull();
    if (!host || !content || !track || !thumb) return;

    const container = document.createElement("diffs-container");
    const shadowRoot = container.attachShadow({ mode: "open" });
    const code = document.createElement("div");
    code.dataset.code = "";
    shadowRoot.append(code);
    content.append(container);

    Object.defineProperties(code, {
      clientWidth: { configurable: true, value: 500 },
      scrollWidth: { configurable: true, value: 1500 },
      scrollLeft: { configurable: true, value: 500, writable: true },
    });
    Object.defineProperty(track, "clientWidth", { configurable: true, value: 500 });
    host.dispatchEvent(new Event("scroll"));

    expect(thumb.hidden).toBe(false);
    expect(Number.parseFloat(thumb.style.width)).toBeCloseTo(166.67, 1);
    expect(Number.parseFloat(thumb.style.transform.slice("translateX(".length))).toBeCloseTo(166.67, 1);
    expect(content.style.paddingBottom).toBe("11px");
  });
});
