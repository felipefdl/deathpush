import { useEffect, useRef } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { useRepositoryStore } from "../../stores/repository-store";
import { useThemeStore } from "../../stores/theme-store";
import { useSettingsStore } from "../../stores/settings-store";
import { getTerminalTheme } from "../../lib/themes/apply-theme";
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
  const result = await invoke<SpawnResult>("terminal_spawn", { cols: term.cols, rows: term.rows });
  sessionIdRef.current = result.id;
  useRepositoryStore.getState().renamePane(paneId, result.shell);
};

export const TerminalInstance = ({ paneId, isActive }: TerminalInstanceProps) => {
  const containerRef = useRef<HTMLDivElement>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitAddonRef = useRef<FitAddon | null>(null);
  const sessionIdRef = useRef(0);
  const exitedRef = useRef(false);

  useEffect(() => {
    if (!containerRef.current) return;

    const { currentTheme } = useThemeStore.getState();
    const theme = getTerminalTheme(currentTheme.colors);
    const termSettings = useSettingsStore.getState().settings.terminal;

    const term = new Terminal({
      theme,
      fontFamily: termSettings.fontFamily,
      fontSize: termSettings.fontSize,
      lineHeight: termSettings.lineHeight,
      cursorBlink: termSettings.cursorBlink,
      cursorStyle: termSettings.cursorStyle,
      allowProposedApi: true,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    term.loadAddon(fitAddon);
    term.loadAddon(webLinksAddon);
    term.open(containerRef.current);

    termRef.current = term;
    fitAddonRef.current = fitAddon;

    requestAnimationFrame(() => {
      fitAddon.fit();
      spawnSession(term, sessionIdRef, paneId);
    });

    const dataDisposable = term.onData((data) => {
      if (exitedRef.current) {
        exitedRef.current = false;
        term.reset();
        const oldId = sessionIdRef.current;
        if (oldId) {
          invoke("terminal_kill", { id: oldId }).then(() => spawnSession(term, sessionIdRef, paneId));
        } else {
          spawnSession(term, sessionIdRef, paneId);
        }
        return;
      }
      if (sessionIdRef.current) {
        invoke("terminal_write", { id: sessionIdRef.current, data });
      }
    });

    const resizeDisposable = term.onResize(({ cols, rows }) => {
      if (sessionIdRef.current) {
        invoke("terminal_resize", { id: sessionIdRef.current, cols, rows });
      }
    });

    const appWindow = getCurrentWebviewWindow();
    const unlistenData = appWindow.listen<TerminalDataEvent>("terminal:data", (event) => {
      if (event.payload.id === sessionIdRef.current) {
        term.write(event.payload.data);
      }
    });

    const unlistenExit = appWindow.listen<number>("terminal:exit", (event) => {
      if (event.payload === sessionIdRef.current) {
        exitedRef.current = true;
      }
    });

    const resizeObserver = new ResizeObserver((entries) => {
      const { width, height } = entries[0].contentRect;
      if (width > 0 && height > 0) {
        fitAddon.fit();
      }
    });
    resizeObserver.observe(containerRef.current);

    return () => {
      resizeObserver.disconnect();
      dataDisposable.dispose();
      resizeDisposable.dispose();
      unlistenData.then((fn) => fn());
      unlistenExit.then((fn) => fn());
      if (sessionIdRef.current) {
        invoke("terminal_kill", { id: sessionIdRef.current });
      }
      sessionIdRef.current = 0;
      term.dispose();
      termRef.current = null;
      fitAddonRef.current = null;
    };
  }, []);

  useEffect(() => {
    if (isActive && fitAddonRef.current && termRef.current) {
      requestAnimationFrame(() => {
        fitAddonRef.current?.fit();
        termRef.current?.focus();
      });
    }
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

  const terminalSettings = useSettingsStore((s) => s.settings.terminal);
  useEffect(() => {
    const term = termRef.current;
    if (!term) return;
    term.options.fontFamily = terminalSettings.fontFamily;
    term.options.fontSize = terminalSettings.fontSize;
    term.options.lineHeight = terminalSettings.lineHeight;
    term.options.cursorBlink = terminalSettings.cursorBlink;
    term.options.cursorStyle = terminalSettings.cursorStyle;
    fitAddonRef.current?.fit();
  }, [terminalSettings]);

  return (
    <div className="terminal-instance-wrapper">
      <div className="terminal-instance" ref={containerRef} />
    </div>
  );
};
