import { afterEach, describe, expect, it, vi } from "vite-plus/test";
import type { TerminalSettings } from "../../stores/settings-store";
import * as terminalInstance from "./terminal-instance";

type SelectionSettings = Pick<TerminalSettings, "rightClickSelectsWord" | "macOptionClickForcesSelection">;
type AttachTerminalSelectionHandlers = (element: HTMLElement, getSettings: () => SelectionSettings) => () => void;
const isAttachTerminalSelectionHandlers = (value: unknown): value is AttachTerminalSelectionHandlers =>
  typeof value === "function";

const getAttach = (): AttachTerminalSelectionHandlers | undefined => {
  const module: object = terminalInstance;
  const attach = "attachTerminalSelectionHandlers" in module ? module.attachTerminalSelectionHandlers : undefined;
  expect(isAttachTerminalSelectionHandlers(attach)).toBe(true);
  return isAttachTerminalSelectionHandlers(attach) ? attach : undefined;
};

afterEach(() => {
  document.body.replaceChildren();
  vi.restoreAllMocks();
});

describe("shouldFocusTerminal", () => {
  it("does not steal focus from the theme picker", () => {
    const overlay = document.createElement("div");
    overlay.className = "theme-picker";
    const input = document.createElement("input");
    overlay.append(input);
    document.body.append(overlay);
    input.focus();

    expect(terminalInstance.shouldFocusTerminal(document.activeElement)).toBe(false);
  });

  it("focuses when nothing else is active", () => {
    expect(terminalInstance.shouldFocusTerminal(document.body)).toBe(true);
    expect(terminalInstance.shouldFocusTerminal(null)).toBe(true);
  });

  it("returns focus when a terminal toolbar button is active", () => {
    const panel = document.createElement("div");
    panel.className = "app-layout-terminal";
    const button = document.createElement("button");
    button.className = "terminal-panel-btn";
    panel.append(button);
    document.body.append(panel);
    button.focus();

    expect(terminalInstance.shouldFocusTerminal(document.activeElement)).toBe(true);
  });

  it("keeps the terminal from losing focus to a toolbar mousedown", () => {
    const event = new MouseEvent("mousedown", { cancelable: true });
    terminalInstance.retainTerminalButtonFocus(event);
    expect(event.defaultPrevented).toBe(true);
  });
});
describe("terminal selection settings", () => {
  it("selects the word under a right click when enabled", () => {
    const attach = getAttach();
    if (!attach) return;

    const element = document.createElement("div");
    const text = document.createTextNode("hello world");
    element.append(text);
    document.body.append(element);
    const range = document.createRange();
    range.setStart(text, 7);
    range.collapse(true);
    const caretRangeFromPoint = vi.fn(() => range);
    Object.defineProperty(document, "caretRangeFromPoint", { configurable: true, value: caretRangeFromPoint });
    const selection = window.getSelection()!;
    const modify = vi.fn();
    Object.defineProperty(selection, "modify", { configurable: true, value: modify });
    const detach = attach(element, () => ({ rightClickSelectsWord: true, macOptionClickForcesSelection: false }));
    const event = new MouseEvent("contextmenu", { bubbles: true, cancelable: true, clientX: 10, clientY: 20 });

    element.dispatchEvent(event);

    expect(event.defaultPrevented).toBe(true);
    expect(caretRangeFromPoint).toHaveBeenCalledWith(10, 20);
    expect(modify).toHaveBeenNthCalledWith(1, "move", "backward", "word");
    expect(modify).toHaveBeenNthCalledWith(2, "extend", "forward", "word");
    detach();
  });

  it("forces native selection for an Option-click when enabled", () => {
    const attach = getAttach();
    if (!attach) return;

    const element = document.createElement("div");
    document.body.append(element);
    const wtermMouseDown = vi.fn();
    element.addEventListener("mousedown", wtermMouseDown);
    const detach = attach(element, () => ({ rightClickSelectsWord: false, macOptionClickForcesSelection: true }));

    element.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, altKey: true, button: 0 }));
    expect(wtermMouseDown).not.toHaveBeenCalled();

    element.dispatchEvent(new MouseEvent("mousedown", { bubbles: true, button: 0 }));
    expect(wtermMouseDown).toHaveBeenCalledTimes(1);
    detach();
  });
});

describe("isUsableTerminalGrid", () => {
  it("rejects the collapsed grid produced when the panel is display none", () => {
    expect(terminalInstance.isUsableTerminalGrid(1, 1)).toBe(false);
    expect(terminalInstance.isUsableTerminalGrid(80, 1)).toBe(false);
  });

  it("accepts a normal terminal size", () => {
    expect(terminalInstance.isUsableTerminalGrid(80, 24)).toBe(true);
  });
});

describe("guardTerminalResize", () => {
  it("does not apply a collapsed resize", () => {
    const resize = vi.fn();
    const term = { resize };
    terminalInstance.guardTerminalResize(term);

    term.resize(1, 1);
    expect(resize).not.toHaveBeenCalled();

    term.resize(80, 24);
    expect(resize).toHaveBeenCalledWith(80, 24);
  });
});
