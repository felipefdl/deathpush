import { cleanup, render, waitFor } from "@solidjs/testing-library";
import { createSignal } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";

const mocks = vi.hoisted(() => ({
  fileConstructed: vi.fn(),
  fileCleaned: vi.fn(),
  fileRendered: vi.fn(),
  fileRerendered: vi.fn(),
  fileOptionsUpdated: vi.fn(),
  editorConstructed: vi.fn(),
  mountingDuringRender: vi.fn(),
}));

vi.mock("@pierre/diffs", () => ({
  FileDiff: class {
    options: Record<string, unknown>;
    constructor(options: Record<string, unknown>) {
      this.options = options;
      mocks.fileConstructed(options);
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
      mocks.fileOptionsUpdated(options);
    }
    rerender() {
      mocks.fileRerendered();
    }
    cleanUp() {
      mocks.fileCleaned();
    }
  },
  parseDiffFromFile: vi.fn(),
  parsePatchFiles: vi.fn(() => [{ files: [{ hunks: [] }] }]),
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
  getFilePatch: vi.fn(async () => "patch"),
  getFileHunks: vi.fn(async () => ({
    hunks: [
      {
        header: "@@ -1,0 +1,1 @@",
        oldStart: 1,
        oldLines: 0,
        newStart: 1,
        newLines: 1,
        lines: [{ content: "new", lineType: "add", oldLineNumber: null, newLineNumber: 1 }],
      },
    ],
  })),
  writeFile: vi.fn(async () => undefined),
}));

vi.mock("../../lib/pierre/worker", () => ({ getPierreWorkerPool: () => ({}) }));
vi.mock("../../lib/pierre/sha", () => ({ sha256Utf8: vi.fn(async (value: string) => `sha:${value}`) }));
vi.mock("../../lib/pierre/flush-registry", () => ({
  flushPath: vi.fn(async () => undefined),
  registerFlusher: () => () => undefined,
  trackPendingFlush: (_path: string, promise: Promise<void>) => promise,
}));

import { settingsStore } from "../../stores/settings-store";
import { PierreFileDiff } from "./pierre-file-diff";
import { getScmSession } from "../../lib/pierre/scm-session-registry";

afterEach(() => {
  cleanup();
  vi.clearAllMocks();
  settingsStore.getState().updateDiff({ layout: "sideBySide", showInlineHunkActions: true });
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

  it("rerenders the mounted diff when switching layouts", async () => {
    settingsStore.getState().updateDiff({ layout: "sideBySide" });
    render(() => <PierreFileDiff path="NOTICE" staged={false} groupKind="workingTree" />);
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(1));

    settingsStore.getState().updateDiff({ layout: "inline" });

    await waitFor(() =>
      expect(mocks.fileOptionsUpdated).toHaveBeenLastCalledWith(expect.objectContaining({ diffStyle: "unified" }))
    );
  });

  it("turns inline hunk actions off and on without remounting", async () => {
    settingsStore.getState().updateDiff({ showInlineHunkActions: false });
    render(() => <PierreFileDiff path="NOTICE" staged={false} groupKind="workingTree" />);
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(1));
    const options = mocks.fileConstructed.mock.calls[0][0] as {
      renderAnnotation: (annotation: { side: "additions"; lineNumber: number }) => HTMLElement | undefined;
    };
    const annotation = { side: "additions" as const, lineNumber: 1 };

    expect(options.renderAnnotation(annotation)).toBeUndefined();
    settingsStore.getState().updateDiff({ showInlineHunkActions: true });
    expect(options.renderAnnotation(annotation)).toBeInstanceOf(HTMLElement);
  });
});

describe("PierreFileDiff disk reload", () => {
  it("stamps a unique persist cache key on each disk-won reload", async () => {
    render(() => <PierreFileDiff path="NOTICE" staged={false} groupKind="workingTree" />);
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(1));

    const handle = getScmSession();
    expect(handle).not.toBeNull();

    handle!.reload(
      {
        path: "NOTICE",
        original: "",
        modified: "second",
        originalLanguage: null,
        fileType: "text",
      },
      "sha-second"
    );
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(2));

    handle!.reload(
      {
        path: "NOTICE",
        original: "",
        modified: "third",
        originalLanguage: null,
        fileType: "text",
      },
      "sha-third"
    );
    await waitFor(() => expect(mocks.fileRendered).toHaveBeenCalledTimes(3));

    const keys = mocks.fileRendered.mock.calls.map(
      (call) => (call[0] as { fileDiff?: { cacheKey?: string } }).fileDiff?.cacheKey
    );
    expect(keys).toEqual(["NOTICE", "NOTICE#1", "NOTICE#2"]);
  });
});
