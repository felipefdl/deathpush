import { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";

import { WebglAddon } from "@xterm/addon-webgl";
import { SearchAddon } from "@xterm/addon-search";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useRepositoryStore } from "../../stores/repository-store";
import { useThemeStore } from "../../stores/theme-store";
import { useSettingsStore } from "../../stores/settings-store";
import { getTerminalTheme } from "../../lib/themes/apply-theme";
import { TerminalSearchBar } from "./terminal-search-bar";
import "@xterm/xterm/css/xterm.css";

interface TerminalDataEvent {
  id: number;
  data: string;
}

interface SpawnResult {
  id: number;
  shell: string;
}

interface TerminalInstanceProps {
  paneId: number;
  isActive: boolean;
}

const spawnSession = async (
  term: Terminal,
  sessionIdRef: React.RefObject<number>,
  paneId: number,
) => {
  const { shellPath } = useSettingsStore.getState().settings.terminal;
  const result = await invoke<SpawnResult>("terminal_spawn", {
    cols: term.cols,
    rows: term.rows,
    shellPath: shellPath || null,
    shellArgs: null,
  });
  sessionIdRef.current = result.id;
  useRepositoryStore.getState().renamePane(paneId, result.shell);
};

export const TerminalInstance = ({ paneId, isActive }: TerminalInstanceProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const searchAddonRef = useRef<SearchAddon | null>(null);
  const sessionIdRef = useRef(0);
  const exitedRef = useRef(false);
  const [showSearch, setShowSearch] = useState(false);

  useEffect(() => {
    if (!containerRef.current) return;

    const container = containerRef.current;
    const { currentTheme } = useThemeStore.getState();
    const theme = getTerminalTheme(currentTheme.colors);
    const termSettings = useSettingsStore.getState().settings.terminal;

    let aborted = false;
    let term: Terminal | null = null;
    let fitAddon: FitAddon | null = null;
    let dataDisposable: { dispose: () => void } | null = null;
    let resizeDisposable: { dispose: () => void } | null = null;
    let unlistenData: Promise<() => void> | null = null;
    let unlistenExit: Promise<() => void> | null = null;
    let resizeObserver: ResizeObserver | null = null;
    let resizeTimer: ReturnType<typeof setTimeout>;

    // We must defer term.open() until BOTH:
    // 1. The custom font is loaded (so CharSizeService measures real metrics)
    // 2. The container has non-zero dimensions (so CharSizeService gets real sizes)
    // If either condition is missing, CharSizeService.measure() returns 0 and
    // FitAddon.fit() bails out, leaving the PTY at default 80x24.
    let fontReady = false;
    let containerVisible = false;
    let initialized = false;

    const initTerminal = () => {
      if (initialized || aborted || !fontReady || !containerVisible) return;
      if (!container.isConnected) return;
      initialized = true;

      term = new Terminal({
        theme,
        fontFamily: termSettings.fontFamily,
        fontSize: termSettings.fontSize,
        lineHeight: termSettings.lineHeight,
        cursorBlink: termSettings.cursorBlink,
        cursorStyle: termSettings.cursorStyle,
        scrollback: termSettings.scrollback,
        allowProposedApi: true,
        macOptionIsMeta: termSettings.macOptionIsMeta,
        cursorInactiveStyle: termSettings.cursorInactiveStyle,
        minimumContrastRatio: termSettings.minimumContrastRatio,
        scrollSensitivity: termSettings.scrollSensitivity,
        fastScrollSensitivity: termSettings.fastScrollSensitivity,
        fontWeight: termSettings.fontWeight,
        fontWeightBold: termSettings.fontWeightBold,
        letterSpacing: termSettings.letterSpacing,
        cursorWidth: termSettings.cursorWidth,
        smoothScrollDuration: termSettings.smoothScrollDuration,
        drawBoldTextInBrightColors: termSettings.drawBoldTextInBrightColors,
        rightClickSelectsWord: termSettings.rightClickSelectsWord,
        macOptionClickForcesSelection: termSettings.macOptionClickForcesSelection,
        altClickMovesCursor: termSettings.altClickMovesCursor,
        wordSeparator: termSettings.wordSeparator,
        tabStopWidth: termSettings.tabStopWidth,
        scrollOnUserInput: termSettings.scrollOnUserInput,
        rescaleOverlappingGlyphs: termSettings.rescaleOverlappingGlyphs,
      });

      fitAddon = new FitAddon();
      term.loadAddon(fitAddon);
      term.loadAddon(new WebLinksAddon());
      term.open(container);

      const unicode11Addon = new Unicode11Addon();
      term.loadAddon(unicode11Addon);
      term.unicode.activeVersion = "11";

      const searchAddon = new SearchAddon();
      term.loadAddon(searchAddon);
      searchAddonRef.current = searchAddon;

      try {
        const webglAddon = new WebglAddon();
        webglAddon.onContextLoss(() => {
          webglAddon.dispose();
        });
        term.loadAddon(webglAddon);

        // Match native terminal color rendering on P3 displays.
        // WebKit applies sRGB->P3 color management to WebGL canvases by default,
        // which desaturates colors compared to native apps like Ghostty.
        const webglCanvas = container.querySelector(".xterm-screen canvas");
        if (webglCanvas) {
          const gl = (webglCanvas as HTMLCanvasElement).getContext("webgl2");
          if (gl && "drawingBufferColorSpace" in gl) {
            (gl as WebGL2RenderingContext).drawingBufferColorSpace = "display-p3";
          }
        }
      } catch {
        // WebGL not available, fall back to canvas renderer
      }

      termRef.current = term;
      fitAddonRef.current = fitAddon;

      const localTerm = term;

      dataDisposable = localTerm.onData((data) => {
        if (exitedRef.current) {
          exitedRef.current = false;
          localTerm.reset();
          const oldId = sessionIdRef.current;
          if (oldId) {
            invoke("terminal_kill", { id: oldId })
              .then(() => spawnSession(localTerm, sessionIdRef, paneId))
              .catch((err) => console.error("terminal_kill failed:", err));
          } else {
            spawnSession(localTerm, sessionIdRef, paneId);
          }
          return;
        }
        if (sessionIdRef.current) {
          invoke("terminal_write", { id: sessionIdRef.current, data }).catch(() => {});
        }
      });

      // Debounce PTY resize notifications to prevent SIGWINCH flooding,
      // but let xterm reflow its buffer immediately (via ResizeObserver below).
      resizeDisposable = localTerm.onResize(({ cols, rows }) => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
          if (sessionIdRef.current) {
            invoke("terminal_resize", { id: sessionIdRef.current, cols, rows }).catch(() => {});
          }
        }, 150);
      });

      const appWindow = getCurrentWebviewWindow();
      unlistenData = appWindow.listen<TerminalDataEvent>("terminal:data", (event) => {
        if (event.payload.id === sessionIdRef.current) {
          localTerm.write(event.payload.data);
        }
      });

      unlistenExit = appWindow.listen<number>("terminal:exit", (event) => {
        if (event.payload === sessionIdRef.current) {
          exitedRef.current = true;
        }
      });

      fitAddon.fit();
      spawnSession(localTerm, sessionIdRef, paneId);
      if (isActive) localTerm.focus();
    };

    // Condition 1: Font loading
    const primaryFont = termSettings.fontFamily.split(",")[0].trim();
    const fontLoad = Promise.all([
      document.fonts.load(`${termSettings.fontSize}px ${primaryFont}`),
      document.fonts.load(`bold ${termSettings.fontSize}px ${primaryFont}`),
    ]);
    const timeout = new Promise<FontFace[]>((resolve) => setTimeout(() => resolve([]), 500));

    Promise.race([fontLoad, timeout]).then(() => {
      fontReady = true;
      initTerminal();
    });

    // Condition 2: Container visibility (non-zero dimensions).
    // Also handles resize after initialization.
    resizeObserver = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      if (width > 0 && height > 0) {
        if (!containerVisible) {
          containerVisible = true;
          initTerminal();
        } else if (fitAddon) {
          fitAddon.fit();
        }
      }
    });
    resizeObserver.observe(container);

    return () => {
      aborted = true;
      clearTimeout(resizeTimer);
      resizeObserver?.disconnect();
      dataDisposable?.dispose();
      resizeDisposable?.dispose();
      unlistenData?.then((fn) => fn());
      unlistenExit?.then((fn) => fn());
      if (sessionIdRef.current) {
        invoke("terminal_kill", { id: sessionIdRef.current }).catch(() => {});
      }
      sessionIdRef.current = 0;
      term?.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
      searchAddonRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (isActive && termRef.current) {
      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
        termRef.current?.refresh(0, termRef.current.rows - 1);
        termRef.current?.focus();
      });
    }
  }, [isActive]);

  useEffect(() => {
    if (!isActive) return;
    const handleFocus = () => {
      termRef.current?.focus();
    };
    window.addEventListener("deathpush:focus-terminal", handleFocus);
    return () => window.removeEventListener("deathpush:focus-terminal", handleFocus);
  }, [isActive]);

  useEffect(() => {
    const interval = setInterval(async () => {
      const sid = sessionIdRef.current;
      if (!sid || exitedRef.current) return;
      const name = await invoke<string>("terminal_foreground_process", { id: sid });
      useRepositoryStore.getState().renamePane(paneId, name);
    }, 1000);
    return () => clearInterval(interval);
  }, [paneId]);

  useEffect(() => {
    const handler = (e: Event) => {
      const { colors } = (e as CustomEvent<{ colors: Record<string, string> }>).detail;
      if (termRef.current) {
        termRef.current.options.theme = getTerminalTheme(colors);
      }
    };
    window.addEventListener("deathpush:theme-applied", handler);
    return () => window.removeEventListener("deathpush:theme-applied", handler);
  }, []);

  useEffect(() => {
    if (!isActive) return;
    const container = containerRef.current;
    if (!container) return;
    const handler = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "f") {
        e.preventDefault();
        e.stopPropagation();
        setShowSearch((prev) => !prev);
      }
    };
    container.addEventListener("keydown", handler, true);
    return () => container.removeEventListener("keydown", handler, true);
  }, [isActive]);

  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    const disposable = term.onSelectionChange(() => {
      const { copyOnSelect } = useSettingsStore.getState().settings.terminal;
      if (copyOnSelect) {
        const selection = term.getSelection();
        if (selection) {
          navigator.clipboard.writeText(selection);
        }
      }
    });
    return () => disposable.dispose();
  }, []);

  const terminalSettings = useSettingsStore((s) => s.settings.terminal);
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    term.options.fontFamily = terminalSettings.fontFamily;
    term.options.fontSize = terminalSettings.fontSize;
    term.options.lineHeight = terminalSettings.lineHeight;
    term.options.cursorBlink = terminalSettings.cursorBlink;
    term.options.cursorStyle = terminalSettings.cursorStyle;
    term.options.scrollback = terminalSettings.scrollback;
    term.options.macOptionIsMeta = terminalSettings.macOptionIsMeta;
    term.options.cursorInactiveStyle = terminalSettings.cursorInactiveStyle;
    term.options.minimumContrastRatio = terminalSettings.minimumContrastRatio;
    term.options.scrollSensitivity = terminalSettings.scrollSensitivity;
    term.options.fastScrollSensitivity = terminalSettings.fastScrollSensitivity;
    term.options.fontWeight = terminalSettings.fontWeight;
    term.options.fontWeightBold = terminalSettings.fontWeightBold;
    term.options.letterSpacing = terminalSettings.letterSpacing;
    term.options.cursorWidth = terminalSettings.cursorWidth;
    term.options.smoothScrollDuration = terminalSettings.smoothScrollDuration;
    term.options.drawBoldTextInBrightColors = terminalSettings.drawBoldTextInBrightColors;
    term.options.rightClickSelectsWord = terminalSettings.rightClickSelectsWord;
    term.options.macOptionClickForcesSelection = terminalSettings.macOptionClickForcesSelection;
    term.options.altClickMovesCursor = terminalSettings.altClickMovesCursor;
    term.options.wordSeparator = terminalSettings.wordSeparator;
    term.options.tabStopWidth = terminalSettings.tabStopWidth;
    term.options.scrollOnUserInput = terminalSettings.scrollOnUserInput;
    term.options.rescaleOverlappingGlyphs = terminalSettings.rescaleOverlappingGlyphs;
    if (containerRef.current) {
      const sat = terminalSettings.colorSaturation;
      containerRef.current.style.filter = sat !== 1 ? `saturate(${sat})` : "";
    }
    fitAddonRef.current?.fit();
    term.refresh(0, term.rows - 1);
  }, [terminalSettings]);

  const bellStyle = useSettingsStore((s) => s.settings.terminal.bellStyle);
  useEffect(() => {
    const term = termRef.current;
    if (!term || bellStyle === "off") return;
    const disposable = term.onBell(() => {
      if (bellStyle === "sound" || bellStyle === "both") {
        const ctx = new AudioContext();
        const osc = ctx.createOscillator();
        const gain = ctx.createGain();
        osc.connect(gain);
        gain.connect(ctx.destination);
        osc.frequency.value = 800;
        gain.gain.value = 0.1;
        osc.start();
        osc.stop(ctx.currentTime + 0.1);
      }
      if (bellStyle === "visual" || bellStyle === "both") {
        const el = containerRef.current;
        if (el) {
          el.classList.add("terminal-bell-flash");
          el.addEventListener("animationend", () => el.classList.remove("terminal-bell-flash"), { once: true });
        }
      }
    });
    return () => disposable.dispose();
  }, [bellStyle]);

  return (
    <div className="terminal-instance-wrapper">
      {showSearch && searchAddonRef.current && (
        <TerminalSearchBar searchAddon={searchAddonRef.current} onClose={() => setShowSearch(false)} />
      )}
      <div className="terminal-instance" ref={containerRef} />
    </div>
  );
};
