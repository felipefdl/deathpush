import { useState, useRef, useEffect, useCallback } from "react";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { invoke } from "@tauri-apps/api/core";
import { useRepositoryStore } from "../../stores/repository-store";

const IS_LINUX = !navigator.userAgent.includes("Macintosh") && !navigator.userAgent.includes("Windows");

interface MenuItem {
  type: "item" | "separator";
  label?: string;
  shortcut?: string;
  event?: string;
  action?: () => void;
  needsRepo?: boolean;
}

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
  { type: "item", label: "Install CLI Tool...", event: "menu:install-cli" },
  { type: "separator" },
  { type: "item", label: "Quit", action: () => invoke("quit_app") },
];

export const LinuxTitleBar = () => {
  if (!IS_LINUX) return null;

  const [menuOpen, setMenuOpen] = useState(false);
  const [isMaximized, setIsMaximized] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const appWindow = getCurrentWebviewWindow();
  const hasRepo = useRepositoryStore((s) => s.status !== null);
  const status = useRepositoryStore((s) => s.status);

  const repoName = status?.root ? status.root.split("/").filter(Boolean).pop() : undefined;
  const branch = status?.headBranch;
  const titleText = repoName
    ? `${repoName}${branch ? ` - ${branch}` : ""}`
    : "DeathPush";

  useEffect(() => {
    let mounted = true;
    appWindow.isMaximized().then((v) => { if (mounted) setIsMaximized(v); });
    const unlisten = appWindow.onResized(() => {
      appWindow.isMaximized().then((v) => { if (mounted) setIsMaximized(v); });
    });
    return () => { mounted = false; unlisten.then((fn) => fn()); };
  }, [appWindow]);

  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) {
        setMenuOpen(false);
      }
    };
    document.addEventListener("mousedown", handler);
    return () => document.removeEventListener("mousedown", handler);
  }, [menuOpen]);

  useEffect(() => {
    if (!menuOpen) return;
    const handler = (e: KeyboardEvent) => {
      if (e.key === "Escape") setMenuOpen(false);
    };
    document.addEventListener("keydown", handler);
    return () => document.removeEventListener("keydown", handler);
  }, [menuOpen]);

  const handleMenuAction = useCallback((item: MenuItem) => {
    setMenuOpen(false);
    if (item.action) {
      item.action();
    } else if (item.event) {
      appWindow.emit(item.event, null);
    }
  }, [appWindow]);

  return (
    <div className="linux-title-bar">
      <div className="linux-title-bar-left">
        <div className="linux-menu-wrapper" ref={menuRef}>
          <button
            className="linux-title-btn linux-menu-btn"
            onClick={() => setMenuOpen(!menuOpen)}
          >
            <span className="codicon codicon-menu" />
          </button>
          {menuOpen && (
            <div
              className="linux-menu-dropdown"
              style={{
                backgroundColor: getComputedStyle(document.documentElement)
                  .getPropertyValue("--vscode-menu-background").trim()
                  || getComputedStyle(document.documentElement)
                    .getPropertyValue("--vscode-editor-background").trim()
                  || "#1e1e1e",
              }}
            >
              {MENU_ITEMS.map((item, i) => {
                if (item.type === "separator") {
                  return <div key={i} className="linux-menu-separator" />;
                }
                const disabled = item.needsRepo === true && !hasRepo;
                return (
                  <button
                    key={i}
                    className="linux-menu-item"
                    disabled={disabled}
                    onClick={() => handleMenuAction(item)}
                  >
                    <span className="linux-menu-label">{item.label}</span>
                    {item.shortcut && (
                      <span className="linux-menu-shortcut">{item.shortcut}</span>
                    )}
                  </button>
                );
              })}
            </div>
          )}
        </div>
        <span className="linux-title-text" data-tauri-drag-region>{titleText}</span>
      </div>
      <div className="linux-title-bar-drag" data-tauri-drag-region />
      <div className="linux-title-bar-right">
        <button className="linux-title-btn" onClick={() => invoke("window_minimize")}>
          <span className="codicon codicon-chrome-minimize" />
        </button>
        <button className="linux-title-btn" onClick={() => invoke("window_maximize")}>
          <span className={`codicon codicon-chrome-${isMaximized ? "restore" : "maximize"}`} />
        </button>
        <button className="linux-title-btn linux-close-btn" onClick={() => invoke("window_close")}>
          <span className="codicon codicon-chrome-close" />
        </button>
      </div>
    </div>
  );
};
