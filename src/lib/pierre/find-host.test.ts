import { afterEach, describe, expect, it } from "vite-plus/test";
import { createPierreFindHost, scanPierreFind } from "./find-host";

const line = (text: string): HTMLElement => {
  const element = document.createElement("span");
  element.setAttribute("data-line", "1");
  element.textContent = text;
  return element;
};

const root = (elements: HTMLElement[]): { querySelectorAll: () => HTMLElement[] } => ({
  querySelectorAll: () => elements,
});

const hosts: ReturnType<typeof createPierreFindHost>[] = [];

const mountHost = (wrapper: HTMLElement, text: string) => {
  document.body.append(wrapper);
  const host = createPierreFindHost({ getRoot: () => root([line(text)]), wrapper });
  hosts.push(host);
  return host;
};

afterEach(() => {
  for (const host of hosts.splice(0)) host.dispose();
  document.body.replaceChildren();
});

describe("scanPierreFind", () => {
  it("returns a range for each case-insensitive match", () => {
    const ranges = scanPierreFind(root([line("alpha Beta alpha")]), "alpha");
    expect(ranges).toHaveLength(2);
    expect(ranges[0].toString()).toBe("alpha");
    expect(ranges[1].toString()).toBe("alpha");
  });

  it("returns no ranges when the needle is missing", () => {
    expect(scanPierreFind(root([line("alpha beta")]), "gamma")).toEqual([]);
  });

  it("returns no ranges for a blank query", () => {
    expect(scanPierreFind(root([line("alpha")]), "   ")).toEqual([]);
  });

  it("maps case-folded offsets back onto the source text", () => {
    const ranges = scanPierreFind(root([line("İstanbul")]), "İ");
    expect(ranges).toHaveLength(1);
    expect(ranges[0].toString()).toBe("İ");
  });

  it("matches context-sensitive lowercase of the whole query", () => {
    const ranges = scanPierreFind(root([line("ΟΣ")]), "ΟΣ");
    expect(ranges).toHaveLength(1);
    expect(ranges[0].toString()).toBe("ΟΣ");
  });
});

describe("createPierreFindHost", () => {
  const findKey = (): KeyboardEvent => new KeyboardEvent("keydown", { key: "f", metaKey: true, cancelable: true });

  it("does not open an arbitrary connected host when focus is outside every pane", () => {
    const hostA = mountHost(document.createElement("div"), "alpha");
    const hostB = mountHost(document.createElement("div"), "beta");
    window.dispatchEvent(findKey());
    expect(hostA.isOpen()).toBe(false);
    expect(hostB.isOpen()).toBe(false);
  });

  it("opens the pane last marked active by pointerdown", () => {
    const wrapA = document.createElement("div");
    const wrapB = document.createElement("div");
    const hostA = mountHost(wrapA, "alpha");
    const hostB = mountHost(wrapB, "beta");
    wrapB.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true }));
    window.dispatchEvent(findKey());
    expect(hostA.isOpen()).toBe(false);
    expect(hostB.isOpen()).toBe(true);
  });

  it("closes the focused open host instead of the first open host", () => {
    const wrapA = document.createElement("div");
    const wrapB = document.createElement("div");
    const hostA = mountHost(wrapA, "alpha");
    const hostB = mountHost(wrapB, "beta");
    hostA.open();
    hostB.open();
    const input = wrapB.querySelector("input");
    expect(input).toBeTruthy();
    input?.focus();
    window.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", cancelable: true }));
    expect(hostA.isOpen()).toBe(true);
    expect(hostB.isOpen()).toBe(false);
  });
});
