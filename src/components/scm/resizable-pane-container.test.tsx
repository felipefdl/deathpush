import { cleanup, fireEvent, render } from "@solidjs/testing-library";
import { flush } from "solid-js";
import { afterEach, beforeEach, describe, expect, it, vi } from "vite-plus/test";
import { layoutStore } from "../../stores/layout-store";
import { ResizablePaneContainer, type PaneDefinition } from "./resizable-pane-container";

const pane = (id: string): PaneDefinition => ({
  id,
  defaultRatio: 1,
  header: (collapsed, onToggle) => (
    <button type="button" data-testid={`toggle-${id}`} onClick={onToggle}>
      {collapsed ? "expand" : "collapse"} {id}
    </button>
  ),
  body: () => <div data-testid={`body-${id}`}>{id}</div>,
});

const flexGrow = (element: Element): number => Number((element as HTMLElement).style.flex.split(" ")[0]);

const expandedWrappers = (container: HTMLElement): Element[] => [
  ...container.querySelectorAll(".resizable-pane-wrapper"),
];

class TestResizeObserver implements ResizeObserver {
  constructor(_callback: ResizeObserverCallback) {}
  observe(_target: Element, _options?: ResizeObserverOptions): void {}
  unobserve(_target: Element): void {}
  disconnect(): void {}
}

describe("ResizablePaneContainer toggle ratios", () => {
  beforeEach(() => {
    vi.stubGlobal("ResizeObserver", TestResizeObserver);
    layoutStore.setState({ collapsedPanes: [] });
  });

  afterEach(() => {
    cleanup();
    layoutStore.setState({ collapsedPanes: [] });
    vi.unstubAllGlobals();
  });

  it("renormalizes remaining pane ratios from post-toggle membership", () => {
    const result = render(() => <ResizablePaneContainer panes={[pane("a"), pane("b"), pane("c")]} />);
    flush();

    const initial = expandedWrappers(result.container);
    expect(initial).toHaveLength(3);
    for (const wrapper of initial) {
      expect(flexGrow(wrapper)).toBeCloseTo(1 / 3);
    }

    fireEvent.click(result.getByTestId("toggle-a"));
    flush();

    expect(result.container.querySelector('[data-testid="body-a"]')).toBeNull();
    const remaining = expandedWrappers(result.container);
    expect(remaining).toHaveLength(2);
    expect(flexGrow(remaining[0])).toBeCloseTo(0.5);
    expect(flexGrow(remaining[1])).toBeCloseTo(0.5);
  });
});
