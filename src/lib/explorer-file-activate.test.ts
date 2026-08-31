import { beforeEach, describe, expect, it } from "vite-plus/test";
import { explorerStore } from "../stores/explorer-store";
import { layoutStore } from "../stores/layout-store";
import { dockTerminalIfCurrentFile, fileTreeClickedFilePath, shouldReloadOpenFile } from "./explorer-file-activate";

describe("fileTreeClickedFilePath", () => {
  it("reads the file path from a tree row click", () => {
    const row = document.createElement("button");
    row.dataset.type = "item";
    row.dataset.itemType = "file";
    row.dataset.itemPath = "README.md";
    const label = document.createElement("span");
    row.append(label);

    expect(fileTreeClickedFilePath({ target: label } as unknown as Event)).toBe("README.md");
  });

  it("ignores directory rows", () => {
    const row = document.createElement("button");
    row.dataset.type = "item";
    row.dataset.itemType = "folder";
    row.dataset.itemPath = "src/";
    const label = document.createElement("span");
    row.append(label);

    expect(fileTreeClickedFilePath({ target: label } as unknown as Event)).toBeNull();
  });
});

describe("dockTerminalIfCurrentFile", () => {
  beforeEach(() => {
    explorerStore.getState().reset();
    layoutStore.setState({ terminalMaximized: true, mainView: "file" });
  });

  it("docks a maximized terminal when the open file is clicked again", () => {
    explorerStore.getState().setSelectedPath("README.md");

    dockTerminalIfCurrentFile("README.md");

    expect(layoutStore.getState().terminalMaximized).toBe(false);
    expect(layoutStore.getState().mainView).toBe("file");
  });

  it("leaves the terminal maximized when a different file is clicked", () => {
    explorerStore.getState().setSelectedPath("README.md");

    dockTerminalIfCurrentFile("vite.config.ts");

    expect(layoutStore.getState().terminalMaximized).toBe(true);
  });
});

describe("shouldReloadOpenFile", () => {
  it("does not reload the file that is already open", () => {
    expect(shouldReloadOpenFile("README.md", "README.md")).toBe(false);
    expect(shouldReloadOpenFile("README.md", "vite.config.ts")).toBe(true);
    expect(shouldReloadOpenFile(null, "README.md")).toBe(true);
  });
});
