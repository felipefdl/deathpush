import { cleanup, render } from "@solidjs/testing-library";
import { createSignal, flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import type { Mock } from "vite-plus/test";
import { explorerStore } from "../../stores/explorer-store";
import { settingsStore } from "../../stores/settings-store";
import { FileTreeHost } from "./file-tree-host";

const treeMocks = vi.hoisted(() => ({
  instances: [] as Array<{
    options: Record<string, unknown>;
    container: HTMLElement;
    cleanUp: Mock;
    render: Mock;
    resetPaths: Mock;
    setGitStatus: Mock;
    setIcons: Mock;
    getItem: Mock;
  }>,
}));

vi.mock("@pierre/trees", () => ({
  FileTree: class {
    cleanUp = vi.fn();
    render = vi.fn();
    resetPaths = vi.fn();
    setGitStatus = vi.fn();
    setIcons = vi.fn();
    subscribe = vi.fn(() => () => undefined);
    getItem = vi.fn(() => null);
    container = document.createElement("file-tree-container");

    constructor(public options: Record<string, unknown>) {
      treeMocks.instances.push(this);
    }

    getFileTreeContainer() {
      return this.container;
    }

    getFocusedPath() {
      return null;
    }

    getSelectedPaths() {
      return [];
    }
  },
  prepareFileTreeInput: vi.fn((paths: readonly string[]) => ({ paths })),
  themeToTreeStyles: vi.fn(() => ({ "--trees-theme-sidebar-bg": "rgb(1, 2, 3)" })),
}));

describe("FileTreeHost", () => {
  afterEach(() => {
    cleanup();
    treeMocks.instances.length = 0;
    explorerStore.getState().reset();
    settingsStore.getState().updateUI({ treeDensity: "compact", treeIcons: "complete" });
  });

  it("mounts Trees with the configured density and icons and cleans it up", () => {
    const result = render(() => <FileTreeHost paths={["src/", "src/index.ts"]} />);
    flush();

    expect(treeMocks.instances).toHaveLength(1);
    expect(treeMocks.instances[0].options).toMatchObject({ density: "compact", icons: "complete" });
    expect(treeMocks.instances[0].render).toHaveBeenCalledWith({
      containerWrapper: result.container.firstElementChild,
    });
    expect((result.container.firstElementChild as HTMLElement).style.getPropertyValue("--trees-theme-sidebar-bg")).toBe(
      "rgb(1, 2, 3)"
    );
    expect(
      (result.container.firstElementChild as HTMLElement).style.getPropertyValue("--trees-focus-ring-width-override")
    ).toBe("0px");

    result.unmount();
    expect(treeMocks.instances[0].cleanUp).toHaveBeenCalledTimes(1);
  });

  it("rebuilds the model when density changes", () => {
    render(() => <FileTreeHost paths={["README.md"]} />);
    flush();

    settingsStore.getState().updateUI({ treeDensity: "default" });
    flush();

    expect(treeMocks.instances).toHaveLength(2);
    expect(treeMocks.instances[0].cleanUp).toHaveBeenCalledTimes(1);
    expect(treeMocks.instances[1].options).toMatchObject({ density: "default", icons: "complete" });
  });

  it("applies files that arrive after an empty first paint", () => {
    let setPaths!: (paths: string[]) => void;
    const Harness = () => {
      const [paths, set] = createSignal<string[]>([]);
      setPaths = set;
      return <FileTreeHost paths={paths()} />;
    };
    render(() => <Harness />);
    flush();

    expect(treeMocks.instances).toHaveLength(1);
    expect(treeMocks.instances[0].resetPaths).not.toHaveBeenCalled();

    setPaths(["README.md"]);
    flush();

    expect(treeMocks.instances[0].resetPaths).toHaveBeenCalledTimes(1);
  });

  it("does not apply explorerStore.selectedPath when selectedPath is omitted", () => {
    explorerStore.getState().setSelectedPath("src/explorer.ts");
    render(() => <FileTreeHost paths={["src/explorer.ts", "src/scm.ts"]} />);
    flush();

    expect(treeMocks.instances[0].options.initialSelectedPaths).toEqual([]);
    expect(treeMocks.instances[0].getItem).not.toHaveBeenCalledWith("src/explorer.ts");
  });

  it("seeds initialSelectedPaths from selectedPath", () => {
    explorerStore.getState().setSelectedPath("src/explorer.ts");
    render(() => <FileTreeHost paths={["src/explorer.ts", "src/scm.ts"]} selectedPath="src/scm.ts" />);
    flush();

    expect(treeMocks.instances[0].options.initialSelectedPaths).toEqual(["src/scm.ts"]);
  });

  it("activates a file on click even when it is already selected", () => {
    const onFileActivate = vi.fn();
    render(() => <FileTreeHost paths={["README.md"]} selectedPath="README.md" onFileActivate={onFileActivate} />);
    flush();

    const row = document.createElement("button");
    row.dataset.type = "item";
    row.dataset.itemType = "file";
    row.dataset.itemPath = "README.md";
    treeMocks.instances[0].container.appendChild(row);
    row.click();

    expect(onFileActivate).toHaveBeenCalledWith("README.md");
  });
});
