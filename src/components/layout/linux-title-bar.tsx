import { createEffect, createSignal, For, onSettled } from "solid-js";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { repositoryStore } from "../../stores/repository-store";
import { useStore } from "../../lib/use-store";
import { IS_LINUX } from "../../lib/platform";

type MenuItem = {
  type: "item" | "separator";
  label?: string;
  shortcut?: string;
  event?: string;
  action?: () => void;
  needsRepo?: boolean;
};

const MENU_ITEMS: MenuItem[] = [
  { type: "item", label: "New Window", shortcut: "Ctrl+N", action: () => invoke("new_window") },
  { type: "item", label: "Open Repository...", shortcut: "Ctrl+O", event: "menu:open-repo" },
  { type: "item", label: "Clone Repository...", event: "menu:clone-repo" },
  { type: "separator" },
  { type: "item", label: "Changes", shortcut: "Ctrl+1", event: "menu:view-changes", needsRepo: true },
  { type: "item", label: "History", shortcut: "Ctrl+2", event: "menu:view-history", needsRepo: true },
  { type: "item", label: "Toggle Diff Mode", shortcut: "Ctrl+Shift+P", event: "menu:toggle-diff", needsRepo: true },
  { type: "separator" },
  { type: "item", label: "Color Theme...", event: "menu:color-theme" },
  { type: "item", label: "File Icon Theme...", event: "menu:icon-theme" },
  { type: "separator" },
  { type: "item", label: "Zoom In", shortcut: "Ctrl+=", event: "menu:zoom-in" },
  { type: "item", label: "Zoom Out", shortcut: "Ctrl+-", event: "menu:zoom-out" },
  { type: "item", label: "Reset Zoom", shortcut: "Ctrl+0", event: "menu:zoom-reset" },
  { type: "separator" },
  { type: "item", label: "Pull", event: "menu:git-pull", needsRepo: true },
  { type: "item", label: "Push", event: "menu:git-push", needsRepo: true },
  { type: "item", label: "Fetch", event: "menu:git-fetch", needsRepo: true },
  { type: "item", label: "Stage All", event: "menu:git-stage-all", needsRepo: true },
  { type: "item", label: "Unstage All", event: "menu:git-unstage-all", needsRepo: true },
  { type: "item", label: "Stash...", event: "menu:git-stash", needsRepo: true },
  { type: "item", label: "Stash Pop", event: "menu:git-stash-pop", needsRepo: true },
  { type: "item", label: "Undo Last Commit", event: "menu:git-undo-commit", needsRepo: true },
  { type: "separator" },
  { type: "item", label: "New Terminal", shortcut: "Ctrl+Shift+J", event: "menu:new-terminal", needsRepo: true },
  { type: "item", label: "Kill Terminal", event: "menu:kill-terminal", needsRepo: true },
  { type: "item", label: "Toggle Terminal", shortcut: "Ctrl+J", event: "menu:toggle-terminal", needsRepo: true },
  { type: "separator" },
  { type: "item", label: "Settings...", shortcut: "Ctrl+,", event: "menu:preferences" },
  { type: "separator" },
  { type: "item", label: "Quit", action: () => invoke("quit_app") },
];

export const LinuxTitleBar = () => {
  if (!IS_LINUX) return null;

  const [menuOpen, setMenuOpen] = createSignal(false);
  const [isMaximized, setIsMaximized] = createSignal(false);
  let menuRef: HTMLDivElement | undefined;
  const appWindow = getCurrentWebviewWindow();
  const hasRepo = useStore(repositoryStore, (s) => s.status !== null);
  const status = useStore(repositoryStore, (s) => s.status);

  const repoName = () => (status()?.root ? status()!.root.split("/").filter(Boolean).pop() : undefined);
  const branch = () => status()?.headBranch;
  const titleText = () => {
    const name = repoName();
    return name ? `${name}${branch() ? ` - ${branch()}` : ""}` : "DeathPush";
  };

  onSettled(() => {
    let mounted = true;
    void appWindow.isMaximized().then((v) => {
      if (mounted) setIsMaximized(v);
    });
    const unlisten = appWindow.onResized(() => {
      void appWindow.isMaximized().then((v) => {
        if (mounted) setIsMaximized(v);
      });
    });
    return () => {
      mounted = false;
      void unlisten.then((fn) => fn());
    };
  });

  createEffect(
    () => menuOpen(),
    (open) => {
      if (!open) return;
      const handler = (e: MouseEvent) => {
        if (menuRef && !menuRef.contains(e.target as Node)) {
          setMenuOpen(false);
        }
      };
      document.addEventListener("mousedown", handler);
      return () => document.removeEventListener("mousedown", handler);
    }
  );

  createEffect(
    () => menuOpen(),
    (open) => {
      if (!open) return;
      const handler = (e: KeyboardEvent) => {
        if (e.key === "Escape") setMenuOpen(false);
      };
      document.addEventListener("keydown", handler);
      return () => document.removeEventListener("keydown", handler);
    }
  );

  const handleMenuAction = (item: MenuItem) => {
    setMenuOpen(false);
    if (item.action) {
      item.action();
    } else if (item.event) {
      void appWindow.emitTo(appWindow.label, item.event, null);
    }
  };

  return (
    <div class="linux-title-bar">
      <div class="linux-title-bar-left">
        <div
          class="linux-menu-wrapper"
          ref={(el) => {
            menuRef = el;
          }}
        >
          <button class="linux-title-btn linux-menu-btn" onClick={() => setMenuOpen(!menuOpen())}>
            <span class="codicon codicon-menu" />
          </button>
          {menuOpen() && (
            <div
              class="linux-menu-dropdown"
              style={{
                "background-color":
                  getComputedStyle(document.documentElement).getPropertyValue("--vscode-menu-background").trim() ||
                  getComputedStyle(document.documentElement).getPropertyValue("--vscode-editor-background").trim() ||
                  "#1e1e1e",
              }}
            >
              <For each={MENU_ITEMS} keyed>
                {(item) =>
                  item.type === "separator" ? (
                    <div class="linux-menu-separator" />
                  ) : (
                    <button
                      class="linux-menu-item"
                      disabled={item.needsRepo === true && !hasRepo()}
                      onClick={() => handleMenuAction(item)}
                    >
                      <span class="linux-menu-label">{item.label}</span>
                      {item.shortcut && <span class="linux-menu-shortcut">{item.shortcut}</span>}
                    </button>
                  )
                }
              </For>
            </div>
          )}
        </div>
        <span class="linux-title-text" data-tauri-drag-region>
          {titleText()}
        </span>
      </div>
      <div class="linux-title-bar-drag" data-tauri-drag-region />
      <div class="linux-title-bar-right">
        <button class="linux-title-btn" onClick={() => invoke("window_minimize")}>
          <span class="codicon codicon-chrome-minimize" />
        </button>
        <button class="linux-title-btn" onClick={() => invoke("window_maximize")}>
          <span class={`codicon codicon-chrome-${isMaximized() ? "restore" : "maximize"}`} />
        </button>
        <button class="linux-title-btn linux-close-btn" onClick={() => invoke("window_close")}>
          <span class="codicon codicon-chrome-close" />
        </button>
      </div>
    </div>
  );
};
