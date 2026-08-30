import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";

const mocks = vi.hoisted(() => ({
  fileConstructed: vi.fn(),
  fileCleaned: vi.fn(),
  fileRendered: vi.fn(),
  editorConstructed: vi.fn(),
  mountingDuringRender: vi.fn(),
}));

vi.mock("@pierre/diffs", () => ({
  FileDiff: class {
    options: Record<string, unknown>;
    constructor(options: Record<string, unknown>) {
      this.options = options;
      mocks.fileConstructed();
    }
    render(props: { containerWrapper: HTMLElement }) {
      mocks.mountingDuringRender(props.containerWrapper.hasAttribute("data-pierre-mounting"));
      mocks.fileRendered(props);
      const onPostRender = this.options.onPostRender as
        | ((node: HTMLElement, instance: unknown, phase: "mount") => void)
        | undefined;
      onPostRender?.(document.createElement("div"), this, "mount");
    }
    setOptions(options: Record<string, unknown>) {
      this.options = options;
    }
    cleanUp() {
      mocks.fileCleaned();
    }
  },
  parseDiffFromFile: vi.fn(),
  parsePatchFiles: vi.fn(),
}));

vi.mock("@pierre/diffs/edit", () => ({
  Editor: class {
    constructor() {
      mocks.editorConstructed();
    }
    edit() {
      return () => undefined;
    }
    getState() {
      return {};
    }
    cleanUp() {}
  },
}));

vi.mock("@tauri-apps/plugin-dialog", () => ({ confirm: vi.fn(async () => true) }));

vi.mock("../../lib/tauri-commands", () => ({
  getFileDiff: vi.fn(async (path: string) => ({
    original: "",
    modified: `contents:${path}`,
    fileType: "text",
  })),
  getFilePatch: vi.fn(async () => ""),
  getFileHunks: vi.fn(async () => ({ hunks: [] })),
  writeFile: vi.fn(async () => undefined),
}));

vi.mock("../../lib/pierre/worker", () => ({ getPierreWorkerPool: () => ({}) }));
vi.mock("../../lib/pierre/sha", () => ({ sha256Utf8: vi.fn(async (value: string) => `sha:${value}`) }));
vi.mock("../../lib/pierre/flush-registry", () => ({
  flushPath: vi.fn(async () => undefined),
  registerFlusher: () => () => undefined,
  trackPendingFlush: (_path: string, promise: Promise<void>) => promise,
}));

import { PierreFileDiff } from "./pierre-file-diff";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
});

describe("PierreFileDiff navigation", () => {
  it("reuses one FileDiff runtime while switching files", async () => {
    let selectPath!: (path: string) => void;
    const Harness = () => {
      const [path, setPath] = createSignal("NOTICE");
      selectPath = setPath;
      return <PierreFileDiff path={path()} staged={false} groupKind="workingTree" />;
    };

    render(() => <Harness />);
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(1));
    selectPath("LICENSE");
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(2));

    expect(mocks.fileConstructed).toHaveBeenCalledTimes(1);
    expect(mocks.editorConstructed).toHaveBeenCalledTimes(1);
    expect(mocks.mountingDuringRender).toHaveBeenCalledTimes(2);
    expect(mocks.mountingDuringRender).toHaveBeenCalledWith(true);
  });
});
