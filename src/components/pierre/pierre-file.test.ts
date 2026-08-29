import { describe, it, expect } from "vite-plus/test";
import { selectionIsInPierreHost } from "./pierre-file";

describe("selectionIsInPierreHost", () => {
  it("accepts a selection inside the nested diffs-container shadow root", () => {
    const root = document.createElement("div");
    const wrapper = document.createElement("div");
    const container = document.createElement("diffs-container");
    const shadow = container.shadowRoot ?? container.attachShadow({ mode: "open" });
    const caret = document.createElement("span");
    shadow.appendChild(caret);
    wrapper.appendChild(container);
    root.appendChild(wrapper);

    expect(root.contains(caret)).toBe(false);
    expect(selectionIsInPierreHost(root, caret)).toBe(true);
  });

  it("rejects a selection outside the host", () => {
    const root = document.createElement("div");
    const outside = document.createElement("span");
    document.body.append(root, outside);
    expect(selectionIsInPierreHost(root, outside)).toBe(false);
    root.remove();
    outside.remove();
  });
});
