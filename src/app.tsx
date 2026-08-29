import { createEffect, createSignal, onSettled } from "solid-js";
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
import { QuickOpen } from "./components/quick-open/quick-open";
import { WelcomeScreen } from "./components/welcome/welcome-screen";
import { LinuxTitleBar } from "./components/layout/linux-title-bar";
import { BootSplash } from "./components/layout/boot-splash";
import { confirm, message } from "@tauri-apps/plugin-dialog";
import { useRepository } from "./hooks/use-repository";
import { useStash } from "./hooks/use-stash";
import { repositoryStore } from "./stores/repository-store";
import { layoutStore } from "./stores/layout-store";
import * as commands from "./lib/tauri-commands";
import { settingsStore } from "./stores/settings-store";
import { explorerStore } from "./stores/explorer-store";
import { themeStore } from "./stores/theme-store";
import { useKeyboardShortcuts } from "./hooks/use-keyboard-shortcuts";
import { toggleTerminal } from "./lib/toggle-terminal";
import { confirmWindowClose } from "./lib/window-close";
import { flushAll } from "./lib/pierre/flush-registry";
import { DEFAULT_DARK_THEME_ID, DEFAULT_LIGHT_THEME_ID } from "./lib/themes/theme-registry";
import { PLATFORM } from "./lib/platform";
import { useStore } from "./lib/use-store";
import "./styles/codicons.css";
import "./styles/scm.css";
import "./styles/history.css";
import "./styles/settings.css";
import "./styles/welcome.css";

const THEME_STORAGE_KEY = "deathpush:theme";

export const App = () => {
  const { openRepo } = useRepository();
  const error = useStore(repositoryStore, (s) => s.error);
  const status = useStore(repositoryStore, (s) => s.status);
  const { setError, setStatus, startOperation, endOperation } = repositoryStore.getState();
  const { saveStash, popStash } = useStash();
  const [showCloneDialog, setShowCloneDialog] = createSignal(false);
  const [showThemePicker, setShowThemePicker] = createSignal(false);
  const [showIconThemePicker, setShowIconThemePicker] = createSignal(false);
  const [showLicensesModal, setShowLicensesModal] = createSignal(false);
  const [showQuickOpen, setShowQuickOpen] = createSignal(false);
  const [initializing, setInitializing] = createSignal(true);

  useKeyboardShortcuts();

  onSettled(() => {
    document.documentElement.dataset.platform = PLATFORM;
  });

  onSettled(() => {
    const init = async () => {
      try {
        const cliPath = await commands.getInitialPath();
        if (cliPath) {
          void openRepo(cliPath);
        }
      } catch {
        // Fall through to welcome screen
      } finally {
        setInitializing(false);
      }
    };
    void init();
  });

  createEffect(
    () => status()?.root,
    (root, previousRoot) => {
      if (root && root !== previousRoot) {
        layoutStore.getState().loadForProject(root);
        explorerStore.getState().clearCache();
      }
    }
  );

  createEffect(
    () => status() !== null,
    (hasRepo) => {
      commands.setRepoMenuEnabled(hasRepo).catch(() => {});
    }
  );

  const handleOpenRepository = async () => {
    const selected = await open({ directory: true, title: "Open Git Repository" });
    if (selected) {
      void openRepo(selected);
    }
  };

  const handleDismissError = () => {
    setError(null);
  };

  onSettled(() => {
    const appWindow = getCurrentWebviewWindow();
    const listeners = [
      appWindow.listen("menu:preferences", () => {
        layoutStore.getState().setMainView("settings");
      }),
      appWindow.listen("menu:open-repo", () => {
        void handleOpenRepository();
      }),
      appWindow.listen("menu:clone-repo", () => {
        setShowCloneDialog(true);
      }),
      appWindow.listen("menu:view-changes", () => {
        layoutStore.getState().setMainView("changes");
      }),
      appWindow.listen("menu:view-history", () => {
        layoutStore.getState().setMainView("history");
      }),
      appWindow.listen("menu:new-terminal", () => {
        const repo = repositoryStore.getState();
        const layout = layoutStore.getState();
        repo.addTerminalGroup();
        layout.setTerminalVisible(true);
        layout.setPanelTab("terminal");
      }),
      appWindow.listen("menu:kill-terminal", () => {
        const repo = repositoryStore.getState();
        if (repo.activeGroupId !== null) {
          repo.removeTerminalGroup(repo.activeGroupId);
        }
      }),
      appWindow.listen("menu:toggle-terminal", () => {
        toggleTerminal();
      }),
      appWindow.listen("menu:toggle-diff", () => {
        const layout = layoutStore.getState();
        layout.setDiffMode(layout.diffMode === "inline" ? "sideBySide" : "inline");
      }),
      appWindow.listen("menu:zoom-in", () => settingsStore.getState().zoomIn()),
      appWindow.listen("menu:zoom-out", () => settingsStore.getState().zoomOut()),
      appWindow.listen("menu:zoom-reset", () => settingsStore.getState().resetZoom()),
      appWindow.listen("menu:color-theme", () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-theme-picker"));
      }),
      appWindow.listen("menu:icon-theme", () => {
        window.dispatchEvent(new CustomEvent("deathpush:open-icon-theme-picker"));
      }),
      appWindow.listen("menu:git-pull", async () => {
        const branch = repositoryStore.getState().status?.headBranch;
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
        const branch = repositoryStore.getState().status?.headBranch;
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
        void saveStash();
      }),
      appWindow.listen("menu:git-stash-pop", () => {
        void popStash(0);
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
      appWindow.listen("menu:quick-open", () => {
        if (repositoryStore.getState().status) {
          setShowQuickOpen(true);
        }
      }),
      appWindow.listen("menu:open-source-licenses", () => {
        setShowLicensesModal(true);
      }),
      appWindow.listen("menu:install-cli", async () => {
        try {
          const cliStatus = await commands.checkCliInstalled();
          if (cliStatus.installed) {
            const shouldUninstall = await confirm(
              "Command line tools 'dp' and 'deathpush' are already installed. Would you like to uninstall them?",
              { title: "Command Line Tool", kind: "warning", okLabel: "Uninstall", cancelLabel: "Cancel" }
            );
            if (!shouldUninstall) return;
            await commands.uninstallCli();
            await message("Command line tools have been uninstalled.", { title: "Command Line Tool" });
          } else {
            const shouldInstall = await confirm(
              "Install dp and deathpush commands to /usr/local/bin so you can open repositories from any terminal.\n\nExamples:\n  dp .\n  deathpush ~/projects/my-repo",
              { title: "Install Command Line Tool", kind: "warning", okLabel: "Install", cancelLabel: "Cancel" }
            );
            if (!shouldInstall) return;
            await commands.installCli();
            await message(
              "Commands dp and deathpush installed successfully. Restart your terminal to start using them.",
              {
                title: "Command Line Tool",
              }
            );
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
      })
    );
    listeners.push(
      appWindow.listen("window:close-requested", async () => {
        await flushAll();
        const confirmed = await confirmWindowClose();
        if (confirmed) await commands.windowConfirmClose();
      })
    );
    return () => {
      listeners.forEach((p) => p.then((fn) => fn()));
    };
  });

  onSettled(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "o") {
        e.preventDefault();
        void handleOpenRepository();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  });

  onSettled(() => {
    const handler = () => setShowThemePicker(true);
    window.addEventListener("deathpush:open-theme-picker", handler);
    return () => window.removeEventListener("deathpush:open-theme-picker", handler);
  });

  onSettled(() => {
    const handler = () => setShowIconThemePicker(true);
    window.addEventListener("deathpush:open-icon-theme-picker", handler);
    return () => window.removeEventListener("deathpush:open-icon-theme-picker", handler);
  });

  onSettled(() => {
    const handler = () => {
      if (repositoryStore.getState().status) {
        setShowQuickOpen(true);
      }
    };
    window.addEventListener("deathpush:open-quick-open", handler);
    return () => window.removeEventListener("deathpush:open-quick-open", handler);
  });

  const uiSettings = useStore(settingsStore, (s) => s.settings.ui);
  createEffect(
    () => uiSettings(),
    (ui) => {
      document.documentElement.style.setProperty("--vscode-font-family", ui.fontFamily);
      document.documentElement.style.setProperty("--vscode-font-size", `${ui.fontSize}px`);
    }
  );

  const zoomLevel = useStore(settingsStore, (s) => s.settings.ui.zoomLevel);
  createEffect(
    () => zoomLevel(),
    (level) => {
      getCurrentWebviewWindow()
        .setZoom(Math.pow(1.2, level))
        .catch(() => {});
    }
  );

  onSettled(() => {
    const mq = window.matchMedia("(prefers-color-scheme: dark)");
    const handler = (e: MediaQueryListEvent) => {
      const stored = localStorage.getItem(THEME_STORAGE_KEY);
      if (stored) return;
      const id = e.matches ? DEFAULT_DARK_THEME_ID : DEFAULT_LIGHT_THEME_ID;
      themeStore.getState().setTheme(id);
    };
    mq.addEventListener("change", handler);
    return () => mq.removeEventListener("change", handler);
  });

  const showWelcome = () => !initializing() && status() === null;

  return (
    <div class="app">
      <LinuxTitleBar />
      {error() && (
        <div class="error-toast" onClick={handleDismissError}>
          <span class="codicon codicon-error" style={{ "margin-right": "6px" }} />
          {error()}
        </div>
      )}
      {showWelcome() ? (
        <WelcomeScreen
          onOpenRepository={handleOpenRepository}
          onCloneRepository={() => setShowCloneDialog(true)}
          onSelectProject={(path) => openRepo(path)}
        />
      ) : status() !== null ? (
        <AppLayout
          sidebar={
            <SidebarView onOpenRepository={handleOpenRepository} onCloneRepository={() => setShowCloneDialog(true)} />
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
      ) : (
        <BootSplash />
      )}
      {showCloneDialog() && <CloneDialog onClose={() => setShowCloneDialog(false)} />}
      {showThemePicker() && <ThemePicker onClose={() => setShowThemePicker(false)} />}
      {showIconThemePicker() && <IconThemePicker onClose={() => setShowIconThemePicker(false)} />}
      {showQuickOpen() && <QuickOpen onClose={() => setShowQuickOpen(false)} />}
      {showLicensesModal() && <LicensesModal onClose={() => setShowLicensesModal(false)} />}
    </div>
  );
};
