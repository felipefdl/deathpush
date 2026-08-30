import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { layoutStore } from "../../stores/layout-store";
import { SidebarView } from "./sidebar-view";

vi.mock("../scm/scm-view", () => ({
  ScmView: () => <div data-testid="changes-view" />,
}));

vi.mock("../explorer/explorer-view", () => ({
  ExplorerView: () => <div data-testid="explorer-view" />,
}));

describe("SidebarView", () => {
  beforeEach(() => {
    layoutStore.setState({ sidebarView: "scm" });
  });

  afterEach(() => cleanup());

  it("keeps visited sidebar trees mounted while switching visibility", () => {
    const result = render(() => <SidebarView onOpenRepository={() => {}} onCloneRepository={() => {}} />);
    flush();

    expect(result.getByTestId("changes-view").parentElement?.hidden).toBe(false);
    expect(result.queryByTestId("explorer-view")).toBeNull();

    fireEvent.click(result.getByText("Explorer"));
    flush();

    expect(result.getByTestId("changes-view").parentElement?.hidden).toBe(true);
    expect(result.getByTestId("explorer-view").parentElement?.hidden).toBe(false);

    fireEvent.click(result.getByText("Changes"));
    flush();

    expect(result.getByTestId("changes-view").parentElement?.hidden).toBe(false);
    expect(result.getByTestId("explorer-view").parentElement?.hidden).toBe(true);
  });
});
