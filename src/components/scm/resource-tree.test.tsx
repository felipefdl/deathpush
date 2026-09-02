import { cleanup, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import type { FileEntry } from "../../lib/git-types";
import { layoutStore } from "../../stores/layout-store";
import type { FileTreeHostProps } from "../trees/file-tree-host";
import { ResourceTree } from "./resource-tree";

const { loadDiff, host } = vi.hoisted(() => ({
  loadDiff: vi.fn(),
  host: { props: undefined as FileTreeHostProps | undefined },
}));

vi.mock("../../hooks/use-diff", () => ({
  useDiff: () => ({ loadDiff, clearDiff: vi.fn() }),
}));

vi.mock("../trees/file-tree-host", () => ({
  FileTreeHost: (props: FileTreeHostProps) => {
    host.props = props;
    return <div data-testid="file-tree-host" />;
  },
}));

const FILE: FileEntry = { path: "src/app.tsx", status: "modified", renamePath: null };

describe("ResourceTree", () => {
  afterEach(() => {
    cleanup();
    loadDiff.mockReset();
    host.props = undefined;
    layoutStore.setState({ mainView: "changes", terminalMaximized: false });
  });

  it("opens the diff on file activate even when the row is already selected", () => {
    layoutStore.setState({ mainView: "history", terminalMaximized: true });
    render(() => <ResourceTree files={[FILE]} groupKind="workingTree" />);
    flush();

    expect(host.props?.selectedPath).toBeUndefined();
    host.props?.onFileActivate?.(FILE.path);

    expect(loadDiff).toHaveBeenCalledWith(FILE.path, false, "workingTree");
    expect(layoutStore.getState().terminalMaximized).toBe(false);
    expect(layoutStore.getState().mainView).toBe("changes");
  });
});
