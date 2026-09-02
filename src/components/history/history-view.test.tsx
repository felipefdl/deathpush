import { cleanup, render } from "@solidjs/testing-library";
import { flush, resetErrorHalt } from "solid-js";
import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import { HistoryView } from "./history-view";

vi.mock("../../lib/session-client", () => ({
  sendIntent: vi.fn(async () => ({ kind: "snapshot", snapshot: {} })),
}));

describe("HistoryView", () => {
  afterEach(() => {
    cleanup();
    resetErrorHalt();
  });

  it("renders commit details without halting reactivity", () => {
    const result = render(() => <HistoryView />);
    flush();
    expect(result.getByText("Select a commit to view details")).toBeTruthy();
  });
});
