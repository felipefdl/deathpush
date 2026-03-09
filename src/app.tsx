import { useEffect, useCallback, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { AppLayout } from "./components/layout/app-layout";
import { CloneDialog } from "./components/layout/clone-dialog";
import { LicensesModal } from "./components/layout/licenses-modal";
import { StatusBar } from "./components/layout/status-bar";
import { DiffViewer } from "./components/diff/diff-viewer";
import { HistoryView } from "./components/history/history-view";
import { MainPanel } from "./components/layout/main-panel";
import { SettingsPage } from "./components/settings/settings-page";
import { FileViewer } from "./components/file-viewer/file-viewer";
import { SidebarView } from "./components/layout/sidebar-view";
import { TerminalPanel } from "./components/terminal/terminal-panel";
import { ThemePicker } from "./components/theme/theme-picker";
import { IconThemePicker } from "./components/theme/icon-theme-picker";
import { WelcomeScreen } from "./components/welcome/welcome-screen";
import { LinuxTitleBar } from "./components/layout/linux-title-bar";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { useRepository } from "./hooks/use-repository";
import { useStash } from "./hooks/use-stash";
import { useRepositoryStore } from "./stores/repository-store";
import { useLayoutStore } from "./stores/layout-store";
import * as commands from "./lib/tauri-commands";
import { useSettingsStore } from "./stores/settings-store";
import { useExplorerStore } from "./stores/explorer-store";
import { useThemeStore } from "./stores/theme-store";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { toggleTerminal } from "./lib/toggle-terminal";
import { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from "./lib/themes/theme-registry";
import "./styles/codicons.css";
import "./styles/scm.css";
import "./styles/history.css";
import "./styles/settings.css";
import "./styles/welcome.css";

const THEME_STORAGE_KEY = "deathpush:theme";

export const App = () => {
  const { openRepo } = useRepository();
  const { error, setError, setStatus, status, startOperation, endOperation } = useRepositoryStore();
  const { saveStash, popStash } = useStash();
  const [showCloneDialog, setShowCloneDialog] = useState(false);
  const [showThemePicker, setShowThemePicker] = useState(false);
  const [showIconThemePicker, setShowIconThemePicker] = useState(false);
  const [showLicensesModal, setShowLicensesModal] = useState(false);
  const [initializing, setInitializing] = useState(true);

  useKeyboardShortcuts();

  useEffect(() => {
    const init = async () => {
      try {
        const cliPath = await commands.getInitialPath();
        if (cliPath) {
          await openRepo(cliPath);
        }
      } catch {
        // Fall through to welcome screen
      } finally {
        setInitializing(false);
      }
    };
    init();
  }, [openRepo]);

  useEffect(() => {
    if (status?.root) {
      useLayoutStore.getState().loadForProject(status.root);
      useExplorerStore.getState().clearCache();
    }
  }, [status?.root]);

  useEffect(() => {
    commands.setRepoMenuEnabled(status !== null).catch(() => {});
  }, [status]);

  const handleOpenRepository = useCallback(async () => {
    const selected = await open({ directory: true, title: "Open Git Repository" });
    if (selected) {
      openRepo(selected);
    }
  }, [openRepo]);

  const handleDismissError = useCallback(() => {
    setError(null);
  }, [setError]);

  useEffect(() => {
    const appWindow = getCurrentWebviewWindow();
    const listeners = [
      appWindow.listen("menu:preferences", () => {
        useLayoutStore.getState().setMainView("settings");
      }),
      appWindow.listen("menu:open-repo", () => {
        handleOpenRepository();
      }),
      appWindow.listen("menu:clone-repo", () => {
        setShowCloneDialog(true);
      }),
      appWindow.listen("menu:view-changes", () => {
        useLayoutStore.getState().setMainView("changes");
      }),
      appWindow.listen("menu:view-history", () => {
        useLayoutStore.getState().setMainView("history");
      }),
      appWindow.listen("menu:new-terminal", () => {
        const repo = useRepositoryStore.getState();
        const layout = useLayoutStore.getState();
        repo.addTerminalGroup();
        layout.setTerminalVisible(true);
        layout.setPanelTab("terminal");
      }),
      appWindow.listen("menu:kill-terminal", () => {
        const repo = useRepositoryStore.getState();
        if (repo.activeGroupId !== null) {
          repo.removeTerminalGroup(repo.activeGroupId);
        }
      }),
      appWindow.listen("menu:toggle-terminal", () => {
        toggleTerminal();
      }),
      appWindow.listen("menu:toggle-diff", () => {
        const layout = useLayoutStore.getState();
        layout.setDiffMode(layout.diffMode === "inline" ? "sideBySide" : "inline");
      }),
      appWindow.listen("menu:zoom-in", () => useSettingsStore.getState().zoomIn()),
      appWindow.listen("menu:zoom-out", () => useSettingsStore.getState().zoomOut()),
      appWindow.listen("menu:zoom-reset", () => useSettingsStore.getState().resetZoom()),
      appWindow.listen("menu:color-theme", () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
      }),
      appWindow.listen("menu:icon-theme", () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
      }),
      appWindow.listen("menu:git-pull", async () => {
        const branch = useRepositoryStore.getState().status?.headBranch;
        if (!branch) return;
        startOperation("pull");
        try {
          const newStatus = await commands.pull("origin", branch);
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        } finally {
          endOperation("pull");
        }
      }),
      appWindow.listen("menu:git-push", async () => {
        const branch = useRepositoryStore.getState().status?.headBranch;
        if (!branch) return;
        startOperation("push");
        try {
          const newStatus = await commands.push("origin", branch);
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        } finally {
          endOperation("push");
        }
      }),
      appWindow.listen("menu:git-fetch", async () => {
        startOperation("fetch");
        try {
          const newStatus = await commands.fetchRemote("origin", true);
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        } finally {
          endOperation("fetch");
        }
      }),
      appWindow.listen("menu:git-stage-all", async () => {
        startOperation("stage");
        try {
          const newStatus = await commands.stageAll();
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        } finally {
          endOperation("stage");
        }
      }),
      appWindow.listen("menu:git-unstage-all", async () => {
        startOperation("unstage");
        try {
          const newStatus = await commands.unstageAll();
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        } finally {
          endOperation("unstage");
        }
      }),
      appWindow.listen("menu:git-stash", () => {
        saveStash();
      }),
      appWindow.listen("menu:git-stash-pop", () => {
        popStash(0);
      }),
      appWindow.listen("menu:git-undo-commit", async () => {
        const confirmed = await confirm("Undo last commit? Changes will be moved back to staging.", {
          title: "Undo Last Commit",
          kind: "warning",
        });
        if (!confirmed) return;
        try {
          const newStatus = await commands.undoLastCommit();
          setStatus(newStatus);
        } catch (err) {
          setError(String(err));
        }
      }),
      appWindow.listen("menu:open-source-licenses", () => {
        setShowLicensesModal(true);
      }),
      appWindow.listen("menu:install-cli", async () => {
        try {
          const status = await commands.checkCliInstalled();
          if (status.installed) {
            const shouldUninstall = await confirm(
              "Command line tools 'dp' and 'deathpush' are already installed. Would you like to uninstall them?",
              { title: "Command Line Tool", kind: "warning", okLabel: "Uninstall", cancelLabel: "Cancel" },
            );
            if (!shouldUninstall) return;
            await commands.uninstallCli();
            await message("Command line tools have been uninstalled.", { title: "Command Line Tool" });
          } else {
            const shouldInstall = await confirm(
              "Install dp and deathpush commands to /usr/local/bin so you can open repositories from any terminal.\n\nExamples:\n  dp .\n  deathpush ~/projects/my-repo",
              { title: "Install Command Line Tool", kind: "warning", okLabel: "Install", cancelLabel: "Cancel" },
            );
            if (!shouldInstall) return;
            await commands.installCli();
            await message("Commands dp and deathpush installed successfully. Restart your terminal to start using them.", { title: "Command Line Tool" });
          }
        } catch (err) {
          if (String(err).includes("Authorization cancelled")) return;
          setError(String(err));
        }
      }),
    ];
    listeners.push(
      appWindow.listen<string>("watcher:error", (event) => {
        setError(event.payload);
      }),
    );
    return () => { listeners.forEach((p) => p.then((fn) => fn())); };
  }, [handleOpenRepository, startOperation, endOperation, setError, setStatus, saveStash, popStash]);

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "o") {
        e.preventDefault();
        handleOpenRepository();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [handleOpenRepository]);

  // Listen for theme picker chord shortcut
  useEffect(() => {
    const handler = () => setShowThemePicker(true);
    window.addEventListener("deathpush:open-theme-picker", handler);
    return () => window.removeEventListener("deathpush:open-theme-picker", handler);
  }, []);

  // Listen for icon theme picker chord shortcut
  useEffect(() => {
    const handler = () => setShowIconThemePicker(true);
    window.addEventListener("deathpush:open-icon-theme-picker", handler);
    return () => window.removeEventListener("deathpush:open-icon-theme-picker", handler);
  }, []);

  // Apply UI font settings reactively
  const uiSettings = useSettingsStore((s) => s.settings.ui);
  useEffect(() => {
    document.documentElement.style.setProperty("--vscode-font-family", uiSettings.fontFamily);
    document.documentElement.style.setProperty("--vscode-font-size", `${uiSettings.fontSize}px`);
  }, [uiSettings]);

  // Apply zoom level to webview
  const zoomLevel = useSettingsStore((s) => s.settings.ui.zoomLevel);
  useEffect(() => {
    getCurrentWebviewWindow().setZoom(Math.pow(1.2, zoomLevel)).catch(() => {});
  }, [zoomLevel]);

  // Auto-switch theme on system preference change (only if no stored preference)
  useEffect(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      if (stored) return;
      const id = e.matches ? DEFAULT_DARK_THEME_ID : DEFAULT_LIGHT_THEME_ID;
      useThemeStore.getState().setTheme(id);
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  }, []);

  const showWelcome = !initializing && status === null;

  return (
    <div className="app">
      <LinuxTitleBar />
      {error && (
        <div className="error-toast" onClick={handleDismissError}>
          <span className="codicon codicon-error" style={{ marginRight: 6 }} />
          {error}
        </div>
      )}
      {showWelcome ? (
        <WelcomeScreen
          onOpenRepository={handleOpenRepository}
          onCloneRepository={() => setShowCloneDialog(true)}
          onSelectProject={(path) => openRepo(path)}
        />
      ) : status !== null ? (
        <AppLayout
          sidebar={
            <SidebarView
              onOpenRepository={handleOpenRepository}
              onCloneRepository={() => setShowCloneDialog(true)}
            />
          }
          main={
            <MainPanel
              changesView={<DiffViewer />}
              historyView={<HistoryView />}
              settingsView={<SettingsPage />}
              fileView={<FileViewer />}
            />
          }
          terminal={<TerminalPanel />}
          statusBar={<StatusBar />}
        />
      ) : null}
      {showCloneDialog && <CloneDialog onClose={() => setShowCloneDialog(false)} />}
      {showThemePicker && <ThemePicker onClose={() => setShowThemePicker(false)} />}
      {showIconThemePicker && <IconThemePicker onClose={() => setShowIconThemePicker(false)} />}
      {showLicensesModal && <LicensesModal onClose={() => setShowLicensesModal(false)} />}
    </div>
  );
};
