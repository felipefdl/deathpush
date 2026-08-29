import { createEffect, createSignal, onSettled } from "solid-js";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { WebglAddon } from "@xterm/addon-webgl";
import { SearchAddon } from "@xterm/addon-search";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { repositoryStore } from "../../stores/repository-store";
import { themeStore } from "../../stores/theme-store";
import { settingsStore } from "../../stores/settings-store";
import { useStore } from "../../lib/use-store";
import { getTerminalTheme } from "../../lib/themes/apply-theme";
import { TerminalSearchBar } from "./terminal-search-bar";
import "@xterm/xterm/css/xterm.css";

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

const spawnSession = async (term: Terminal, session: { id: number }, paneId: number) => {
  const { shellPath } = settingsStore.getState().settings.terminal;
  const result = await invoke<SpawnResult>("terminal_spawn", {
    cols: term.cols,
    rows: term.rows,
    shellPath: shellPath || null,
    shellArgs: null,
  });
  session.id = result.id;
  repositoryStore.getState().renamePane(paneId, result.shell);
};

export const TerminalInstance = (props: TerminalInstanceProps) => {
  const [showSearch, setShowSearch] = createSignal(false);
  const [searchAddon, setSearchAddon] = createSignal<SearchAddon | undefined>();
  const [termReady, setTermReady] = createSignal(false);
  const terminalSettings = useStore(settingsStore, (s) => s.settings.terminal);
  const bellStyle = useStore(settingsStore, (s) => s.settings.terminal.bellStyle);

  let containerEl: HTMLDivElement | undefined;
  let term: Terminal | undefined;
  let fitAddon: FitAddon | undefined;
  const session = { id: 0 };
  let exited = false;

  onSettled(() => {
    const container = containerEl;
    if (!container) return;

    const { currentTheme } = themeStore.getState();
    const theme = getTerminalTheme(currentTheme.colors);
    const termSettings = settingsStore.getState().settings.terminal;

    let aborted = false;
    let dataDisposable: { dispose: () => void } | undefined;
    let resizeDisposable: { dispose: () => void } | undefined;
    let unlistenData: Promise<() => void> | undefined;
    let unlistenExit: Promise<() => void> | undefined;
    let resizeObserver: ResizeObserver | undefined;
    let resizeTimer: ReturnType<typeof setTimeout> | undefined;

    // Defer term.open() until the custom font is loaded and the container has
    // non-zero size, otherwise FitAddon bails and the PTY stays at 80x24.
    let fontReady = false;
    let containerVisible = false;
    let initialized = false;

    const initTerminal = () => {
      if (initialized || aborted || !fontReady || !containerVisible) return;
      if (!container.isConnected) return;
      initialized = true;

      const nextTerm = new Terminal({
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

      const nextFit = new FitAddon();
      nextTerm.loadAddon(nextFit);
      nextTerm.loadAddon(new WebLinksAddon());
      nextTerm.open(container);

      const unicode11Addon = new Unicode11Addon();
      nextTerm.loadAddon(unicode11Addon);
      nextTerm.unicode.activeVersion = "11";

      const nextSearch = new SearchAddon();
      nextTerm.loadAddon(nextSearch);
      setSearchAddon(nextSearch);

      try {
        const webglAddon = new WebglAddon();
        webglAddon.onContextLoss(() => {
          webglAddon.dispose();
        });
        nextTerm.loadAddon(webglAddon);

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

      term = nextTerm;
      fitAddon = nextFit;

      const sat = termSettings.colorSaturation;
      container.style.filter = sat !== 1 ? `saturate(${sat})` : "";

      dataDisposable = nextTerm.onData((data) => {
        if (exited) {
          exited = false;
          nextTerm.reset();
          const oldId = session.id;
          if (oldId) {
            invoke("terminal_kill", { id: oldId })
              .then(() => spawnSession(nextTerm, session, props.paneId))
              .catch((err) => console.error("terminal_kill failed:", err));
          } else {
            void spawnSession(nextTerm, session, props.paneId);
          }
          return;
        }
        if (session.id) {
          invoke("terminal_write", { id: session.id, data }).catch(() => {});
        }
      });

      resizeDisposable = nextTerm.onResize(({ cols, rows }) => {
        clearTimeout(resizeTimer);
        resizeTimer = setTimeout(() => {
          if (session.id) {
            invoke("terminal_resize", { id: session.id, cols, rows }).catch(() => {});
          }
        }, 150);
      });

      const appWindow = getCurrentWebviewWindow();
      unlistenData = appWindow.listen<TerminalDataEvent>("terminal:data", (event) => {
        if (event.payload.id === session.id) {
          nextTerm.write(event.payload.data);
        }
      });

      unlistenExit = appWindow.listen<number>("terminal:exit", (event) => {
        if (event.payload === session.id) {
          exited = true;
        }
      });

      nextFit.fit();
      void spawnSession(nextTerm, session, props.paneId);
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
      initTerminal();
    });

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
      void unlistenData?.then((fn) => fn());
      void unlistenExit?.then((fn) => fn());
      if (session.id) {
        invoke("terminal_kill", { id: session.id }).catch(() => {});
      }
      session.id = 0;
      term?.dispose();
      term = undefined;
      fitAddon = undefined;
      setSearchAddon(undefined);
      setTermReady(false);
    };
  });

  createEffect(
    () => [props.isActive, termReady()] as const,
    ([active, ready]) => {
      if (active && ready && term) {
        requestAnimationFrame(() => {
          fitAddon?.fit();
          term?.refresh(0, term.rows - 1);
          term?.focus();
        });
      }
    }
  );

  createEffect(
    () => props.isActive,
    (active) => {
      if (!active) return;
      const handleFocus = () => {
        term?.focus();
      };
      window.addEventListener("deathpush:focus-terminal", handleFocus);
      return () => window.removeEventListener("deathpush:focus-terminal", handleFocus);
    }
  );

  createEffect(
    () => props.paneId,
    (paneId) => {
      const interval = setInterval(async () => {
        const sid = session.id;
        if (!sid || exited) return;
        const name = await invoke<string>("terminal_foreground_process", { id: sid });
        repositoryStore.getState().renamePane(paneId, name);
      }, 1000);
      return () => clearInterval(interval);
    }
  );

  onSettled(() => {
    const handler = (e: Event) => {
      const { colors } = (e as CustomEvent<{ colors: Record<string, string> }>).detail;
      if (term) {
        term.options.theme = getTerminalTheme(colors);
      }
    };
    window.addEventListener("deathpush:theme-applied", handler);
    return () => window.removeEventListener("deathpush:theme-applied", handler);
  });

  createEffect(
    () => props.isActive,
    (active) => {
      if (!active) return;
      const container = containerEl;
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
    }
  );

  createEffect(
    () => termReady(),
    (ready) => {
      if (!ready || !term) return;
      const disposable = term.onSelectionChange(() => {
        const { copyOnSelect } = settingsStore.getState().settings.terminal;
        if (copyOnSelect) {
          const selection = term?.getSelection();
          if (selection) {
            void navigator.clipboard.writeText(selection);
          }
        }
      });
      return () => disposable.dispose();
    }
  );

  createEffect(
    () => [termReady(), terminalSettings()] as const,
    ([ready, settings]) => {
      if (!ready || !term) return;
      term.options.fontFamily = settings.fontFamily;
      term.options.fontSize = settings.fontSize;
      term.options.lineHeight = settings.lineHeight;
      term.options.cursorBlink = settings.cursorBlink;
      term.options.cursorStyle = settings.cursorStyle;
      term.options.scrollback = settings.scrollback;
      term.options.macOptionIsMeta = settings.macOptionIsMeta;
      term.options.cursorInactiveStyle = settings.cursorInactiveStyle;
      term.options.minimumContrastRatio = settings.minimumContrastRatio;
      term.options.scrollSensitivity = settings.scrollSensitivity;
      term.options.fastScrollSensitivity = settings.fastScrollSensitivity;
      term.options.fontWeight = settings.fontWeight;
      term.options.fontWeightBold = settings.fontWeightBold;
      term.options.letterSpacing = settings.letterSpacing;
      term.options.cursorWidth = settings.cursorWidth;
      term.options.smoothScrollDuration = settings.smoothScrollDuration;
      term.options.drawBoldTextInBrightColors = settings.drawBoldTextInBrightColors;
      term.options.rightClickSelectsWord = settings.rightClickSelectsWord;
      term.options.macOptionClickForcesSelection = settings.macOptionClickForcesSelection;
      term.options.altClickMovesCursor = settings.altClickMovesCursor;
      term.options.wordSeparator = settings.wordSeparator;
      term.options.tabStopWidth = settings.tabStopWidth;
      term.options.scrollOnUserInput = settings.scrollOnUserInput;
      term.options.rescaleOverlappingGlyphs = settings.rescaleOverlappingGlyphs;
      if (containerEl) {
        const sat = settings.colorSaturation;
        containerEl.style.filter = sat !== 1 ? `saturate(${sat})` : "";
      }
      fitAddon?.fit();
      term.refresh(0, term.rows - 1);
    }
  );

  createEffect(
    () => [termReady(), bellStyle()] as const,
    ([ready, style]) => {
      if (!ready || !term || style === "off") return;
      const disposable = term.onBell(() => {
        if (style === "sound" || style === "both") {
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
        if (style === "visual" || style === "both") {
          const el = containerEl;
          if (el) {
            el.classList.add("terminal-bell-flash");
            el.addEventListener("animationend", () => el.classList.remove("terminal-bell-flash"), {
              once: true,
            });
          }
        }
      });
      return () => disposable.dispose();
    }
  );

  return (
    <div class="terminal-instance-wrapper">
      {showSearch() && searchAddon() ? (
        <TerminalSearchBar searchAddon={searchAddon()!} onClose={() => setShowSearch(false)} />
      ) : null}
      <div
        class="terminal-instance"
        ref={(el) => {
          containerEl = el;
        }}
      />
    </div>
  );
};
