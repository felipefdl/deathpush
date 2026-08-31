import { createEffect, createSignal, onSettled } from "solid-js";
import { WTerm } from "@wterm/dom";
import { GhosttyCore } from "@wterm/ghostty";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { repositoryStore } from "../../stores/repository-store";
import { themeStore } from "../../stores/theme-store";
import { type TerminalSettings, settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { getTerminalTheme, type TerminalTheme } from "../../lib/themes/apply-theme";
import { closeExitedTerminalPane } from "../../lib/close-exited-terminal-pane";
import "@wterm/dom/src/terminal.css";

type TerminalDataEvent = {
  id: number;
  data: string;
};

type SpawnResult = {
  id: number;
  shell: string;
};

type TerminalInstanceProps = {
  paneId: number;
  isActive: boolean;
};

type WTermMetricsAdapter = {
  _charWidth: number;
  _rowHeight: number;
};

const ANSI_THEME_KEYS = [
  "black",
  "red",
  "green",
  "yellow",
  "blue",
  "magenta",
  "cyan",
  "white",
  "brightBlack",
  "brightRed",
  "brightGreen",
  "brightYellow",
  "brightBlue",
  "brightMagenta",
  "brightCyan",
  "brightWhite",
] as const;

const applyTerminalTheme = (element: HTMLElement, theme: TerminalTheme): void => {
  element.style.setProperty("--term-bg", theme.background);
  element.style.setProperty("--term-fg", theme.foreground);
  element.style.setProperty("--term-cursor", theme.cursor);
  element.style.setProperty("--term-selection-background", theme.selectionBackground);
  ANSI_THEME_KEYS.forEach((key, index) => {
    element.style.setProperty(`--term-color-${index}`, theme[key]);
  });
};

const applyGhosttyPalette = (term: WTerm, theme: TerminalTheme): void => {
  const palette = ANSI_THEME_KEYS.map((key, index) => `${index};${theme[key]}`).join(";");
  term.write(`\x1b]4;${palette}\x1b\\`);
};

export const isUsableTerminalGrid = (cols: number, rows: number): boolean => cols >= 2 && rows >= 2;

export const guardTerminalResize = (term: { resize: (cols: number, rows: number) => void }): void => {
  const resize = term.resize.bind(term);
  term.resize = (cols: number, rows: number): void => {
    if (!isUsableTerminalGrid(cols, rows)) return;
    resize(cols, rows);
  };
};

const fitTerminal = (term: WTerm): void => {
  const probe = document.createElement("span");
  probe.textContent = "W";
  probe.style.cssText = "position:absolute;visibility:hidden;display:block;width:max-content;white-space:pre";
  term.element.appendChild(probe);
  const { width: charWidth, height } = probe.getBoundingClientRect();
  probe.remove();
  if (charWidth === 0 || height === 0) return;

  const rowHeight = Math.ceil(height);
  term.element.style.setProperty("--term-row-height", `${rowHeight}px`);

  // WTerm 0.3.4 has no public fit API. Keep its version pinned while this adapter updates its measured cell size.
  const metrics = term as unknown as WTermMetricsAdapter;
  metrics._charWidth = charWidth;
  metrics._rowHeight = rowHeight;

  const styles = getComputedStyle(term.element);
  const width =
    term.element.clientWidth -
    (Number.parseFloat(styles.paddingLeft) || 0) -
    (Number.parseFloat(styles.paddingRight) || 0);
  const heightAvailable =
    term.element.clientHeight -
    (Number.parseFloat(styles.paddingTop) || 0) -
    (Number.parseFloat(styles.paddingBottom) || 0);
  const cols = Math.floor(width / charWidth);
  const rows = Math.floor(heightAvailable / rowHeight);
  if (!isUsableTerminalGrid(cols, rows)) return;
  term.resize(cols, rows);
};

const applyTerminalSettings = (element: HTMLElement, settings: TerminalSettings): void => {
  element.style.setProperty("--term-font-family", settings.fontFamily);
  element.style.setProperty("--term-font-size", `${settings.fontSize}px`);
  element.style.setProperty("--term-line-height", String(settings.lineHeight));
  element.style.setProperty("--term-font-weight", String(settings.fontWeight));
  element.style.setProperty("--term-font-weight-bold", String(settings.fontWeightBold));
  element.style.setProperty("--term-letter-spacing", `${settings.letterSpacing}px`);
  element.style.setProperty("--term-cursor-width", `${settings.cursorWidth}px`);
  element.style.filter = settings.colorSaturation !== 1 ? `saturate(${settings.colorSaturation})` : "";
  element.classList.toggle("cursor-blink", settings.cursorBlink);
  element.dataset.cursorStyle = settings.cursorStyle;
  element.dataset.cursorInactiveStyle = settings.cursorInactiveStyle;
};
const ringBell = (element: HTMLElement): void => {
  const { bellStyle } = settingsStore.getState().settings.terminal;
  if (bellStyle === "off") return;

  if (bellStyle === "sound" || bellStyle === "both") {
    const context = new AudioContext();
    const oscillator = context.createOscillator();
    const gain = context.createGain();
    oscillator.connect(gain);
    gain.connect(context.destination);
    oscillator.frequency.value = 800;
    gain.gain.value = 0.1;
    oscillator.addEventListener("ended", () => void context.close(), { once: true });
    oscillator.start();
    oscillator.stop(context.currentTime + 0.1);
  }

  if (bellStyle === "visual" || bellStyle === "both") {
    element.classList.add("terminal-bell-flash");
    element.addEventListener("animationend", () => element.classList.remove("terminal-bell-flash"), { once: true });
  }
};

const spawnSession = async (term: WTerm, session: { id: number }, paneID: number): Promise<void> => {
  const { shellPath } = settingsStore.getState().settings.terminal;
  const result = await invoke<SpawnResult>("terminal_spawn", {
    cols: term.cols,
    rows: term.rows,
    shellPath: shellPath || null,
    shellArgs: null,
  });
  session.id = result.id;
  repositoryStore.getState().renamePane(paneID, result.shell);
};

export const shouldFocusTerminal = (activeElement: Element | null): boolean =>
  !activeElement ||
  activeElement === document.body ||
  !!activeElement.closest(".terminal-instance, .terminal-instance-wrapper, .app-layout-terminal");

export const requestTerminalFocus = (): void => {
  window.dispatchEvent(new CustomEvent("deathpush:focus-terminal"));
};

export const retainTerminalButtonFocus = (event: MouseEvent): void => {
  event.preventDefault();
};

type TerminalSelectionSettings = Pick<TerminalSettings, "rightClickSelectsWord" | "macOptionClickForcesSelection">;

const selectWordAtPoint = (element: HTMLElement, x: number, y: number): boolean => {
  const range = document.caretRangeFromPoint(x, y);
  const selection = window.getSelection();
  if (!range || !selection || !element.contains(range.startContainer)) return false;

  selection.removeAllRanges();
  selection.addRange(range);
  selection.modify("move", "backward", "word");
  selection.modify("extend", "forward", "word");
  return true;
};

export const attachTerminalSelectionHandlers = (
  element: HTMLElement,
  getSettings: () => TerminalSelectionSettings
): (() => void) => {
  const handleContextMenu = (event: MouseEvent): void => {
    if (!getSettings().rightClickSelectsWord) return;
    if (selectWordAtPoint(element, event.clientX, event.clientY)) event.preventDefault();
  };
  const handleMouseDown = (event: MouseEvent): void => {
    if (event.button === 0 && event.altKey && getSettings().macOptionClickForcesSelection) {
      event.stopImmediatePropagation();
    }
  };

  element.addEventListener("contextmenu", handleContextMenu);
  element.addEventListener("mousedown", handleMouseDown, true);
  return () => {
    element.removeEventListener("contextmenu", handleContextMenu);
    element.removeEventListener("mousedown", handleMouseDown, true);
  };
};

export const TerminalInstance = (props: TerminalInstanceProps) => {
  const [termReady, setTermReady] = createSignal(false);
  const terminalSettings = useStore(settingsStore, (state) => state.settings.terminal);

  let containerEl: HTMLDivElement | undefined;
  let term: WTerm | undefined;
  const session = { id: 0 };
  let terminalMetricsSignature = "";
  let fontLoadGeneration = 0;

  onSettled(() => {
    const container = containerEl;
    if (!container) return;
    const detachSelectionHandlers = attachTerminalSelectionHandlers(
      container,
      () => settingsStore.getState().settings.terminal
    );

    const termSettings = settingsStore.getState().settings.terminal;
    const terminalTheme = getTerminalTheme(themeStore.getState().currentTheme.colors);
    applyTerminalTheme(container, terminalTheme);
    applyTerminalSettings(container, termSettings);

    let aborted = false;
    let unlistenData: Promise<() => void> | undefined;
    let unlistenExit: Promise<() => void> | undefined;
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;
    let visibilityObserver: ResizeObserver | undefined;
    let fontReady = false;
    let containerVisible = false;
    let initialized = false;
    let sessionSpawned = false;

    const spawnWhenFitted = (): void => {
      if (sessionSpawned || aborted || !term) return;
      fitTerminal(term);
      if (!isUsableTerminalGrid(term.cols, term.rows)) return;
      sessionSpawned = true;
      void spawnSession(term, session, props.paneId);
    };

    const initTerminal = async (): Promise<void> => {
      if (initialized || aborted || !fontReady || !containerVisible || !container.isConnected) return;
      initialized = true;
      let core: GhosttyCore;
      try {
        core = await GhosttyCore.load({
          foregroundColor: terminalTheme.foreground,
          backgroundColor: terminalTheme.background,
          scrollbackLimit: termSettings.scrollback * 1024,
        });
      } catch (error) {
        console.error("Ghostty core initialization failed:", error);
        return;
      }
      if (aborted) return;

      const nextTerm = new WTerm(container, {
        core,
        cursorBlink: termSettings.cursorBlink,
        onData: (data) => {
          if (session.id) {
            invoke("terminal_write", { id: session.id, data }).catch(() => {});
          }
        },
        onResize: (cols, rows) => {
          if (!isUsableTerminalGrid(cols, rows)) return;
          clearTimeout(resizeTimer);
          resizeTimer = setTimeout(() => {
            if (session.id) {
              invoke("terminal_resize", { id: session.id, cols, rows }).catch(() => {});
            }
          }, 150);
        },
      });
      guardTerminalResize(nextTerm);
      term = nextTerm;

      try {
        await nextTerm.init();
      } catch (error) {
        console.error("wterm initialization failed:", error);
        return;
      }

      if (aborted) {
        nextTerm.destroy();
        return;
      }

      applyGhosttyPalette(nextTerm, terminalTheme);

      const appWindow = getCurrentWebviewWindow();
      unlistenData = appWindow.listen<TerminalDataEvent>("terminal:data", (event) => {
        if (event.payload.id !== session.id) return;
        if (event.payload.data.includes("\x07")) ringBell(container);
        nextTerm.write(event.payload.data);
      });

      unlistenExit = appWindow.listen<number>("terminal:exit", (event) => {
        if (event.payload === session.id) closeExitedTerminalPane(props.paneId);
      });

      spawnWhenFitted();
      if (props.isActive) nextTerm.focus();
      setTermReady(true);
    };

    const primaryFont = termSettings.fontFamily.split(",")[0].trim();
    const fontLoad = Promise.all([
      document.fonts.load(`${termSettings.fontSize}px ${primaryFont}`),
      document.fonts.load(`bold ${termSettings.fontSize}px ${primaryFont}`),
    ]);
    const timeout = new Promise<FontFace[]>((resolve) => setTimeout(() => resolve([]), 500));

    void Promise.race([fontLoad, timeout]).then(() => {
      fontReady = true;
      void initTerminal();
    });

    const considerVisible = (width: number, height: number): void => {
      if (containerVisible || width <= 0 || height <= 0) return;
      containerVisible = true;
      void initTerminal();
    };
    visibilityObserver = new ResizeObserver((entries) => {
      considerVisible(entries[0].contentRect.width, entries[0].contentRect.height);
      spawnWhenFitted();
    });
    visibilityObserver.observe(container);
    considerVisible(container.clientWidth, container.clientHeight);

    const handleSelectionChange = (): void => {
      const { copyOnSelect } = settingsStore.getState().settings.terminal;
      if (!copyOnSelect) return;
      const selection = window.getSelection();
      if (!selection || selection.isCollapsed || !selection.anchorNode || !container.contains(selection.anchorNode))
        return;
      void navigator.clipboard.writeText(selection.toString());
    };
    document.addEventListener("selectionchange", handleSelectionChange);

    return () => {
      aborted = true;
      clearTimeout(resizeTimer);
      visibilityObserver?.disconnect();
      document.removeEventListener("selectionchange", handleSelectionChange);
      detachSelectionHandlers();
      void unlistenData?.then((unlisten) => unlisten());
      void unlistenExit?.then((unlisten) => unlisten());
      if (session.id) invoke("terminal_kill", { id: session.id }).catch(() => {});
      session.id = 0;
      term?.destroy();
      term = undefined;
    };
  });

  createEffect(
    () => props.isActive && termReady(),
    (shouldFocus) => {
      if (!shouldFocus || !shouldFocusTerminal(document.activeElement)) return;
      requestAnimationFrame(() => {
        if (shouldFocusTerminal(document.activeElement)) term?.focus();
      });
    }
  );

  createEffect(
    () => props.isActive,
    (active) => {
      if (!active) return;
      const handleFocus = () => {
        if (shouldFocusTerminal(document.activeElement)) term?.focus();
      };
      window.addEventListener("deathpush:focus-terminal", handleFocus);
      return () => window.removeEventListener("deathpush:focus-terminal", handleFocus);
    }
  );

  createEffect(
    () => props.paneId,
    (paneID) => {
      const interval = setInterval(async () => {
        const sessionID = session.id;
        if (!sessionID) return;
        const name = await invoke<string>("terminal_foreground_process", { id: sessionID });
        repositoryStore.getState().renamePane(paneID, name);
      }, 1000);
      return () => clearInterval(interval);
    }
  );

  onSettled(() => {
    const handleTheme = (event: Event): void => {
      const { colors } = (event as CustomEvent<{ colors: Record<string, string> }>).detail;
      const terminalTheme = getTerminalTheme(colors);
      if (containerEl) applyTerminalTheme(containerEl, terminalTheme);
      if (term) applyGhosttyPalette(term, terminalTheme);
    };
    window.addEventListener("deathpush:theme-applied", handleTheme);
    return () => window.removeEventListener("deathpush:theme-applied", handleTheme);
  });

  createEffect(
    () => [termReady(), terminalSettings()] as const,
    ([ready, settings]) => {
      if (!ready || !containerEl) return;
      applyTerminalSettings(containerEl, settings);

      const metricsSignature = [
        settings.fontFamily,
        settings.fontSize,
        settings.lineHeight,
        settings.letterSpacing,
        settings.fontWeight,
        settings.fontWeightBold,
      ].join("\0");
      if (metricsSignature === terminalMetricsSignature) return;
      terminalMetricsSignature = metricsSignature;

      const generation = ++fontLoadGeneration;
      const fit = (): void => {
        if (generation === fontLoadGeneration && term) fitTerminal(term);
      };
      requestAnimationFrame(fit);

      const primaryFont = settings.fontFamily.split(",")[0].trim();
      void document.fonts.load(`${settings.fontSize}px ${primaryFont}`).then(() => requestAnimationFrame(fit));
    }
  );

  return (
    <div class="terminal-instance-wrapper">
      <div
        class="terminal-instance"
        ref={(element) => {
          containerEl = element;
        }}
      />
    </div>
  );
};
