# DeathPush - Implementation Progress

## Phase 1: Scaffolding
- [x] Create Tauri v2 project with React + TypeScript
- [x] Configure Cargo.toml with all Rust dependencies
- [x] Configure package.json with all npm dependencies
- [x] Set up oxlintrc.json, justfile, vitest.config.ts
- [x] Create .gitignore, AGENTS.md, README.md
- [x] Install npm dependencies
- [x] Create directory structure
- [x] Verify cargo check passes
- [x] git init

## Phase 2: Rust Git Core
- [x] error.rs, types.rs
- [x] git/repository.rs (git2 wrapper)
- [x] git/status.rs (status classification)
- [x] git/cli.rs (CLI runner)
- [x] git/diff.rs (blob reads)
- [x] git/branch.rs (branch operations)
- [x] git/remote.rs (push/pull/fetch)
- [x] git/commit.rs (stage/unstage/commit)
- [x] git/watcher.rs (FS watcher)
- [x] commands/ (all Tauri commands)
- [x] lib.rs + main.rs

## Phase 3: SCM View (Frontend)
- [x] CSS variables + codicon font
- [x] Zustand store
- [x] Typed Tauri command wrappers
- [x] App layout
- [x] SCM view with resource groups
- [x] Status colors and icons

## Phase 4: Staging + Commit
- [x] Stage/unstage/discard actions
- [x] Commit input
- [x] Stage All / Unstage All

## Phase 5: Monaco Diff Viewer
- [x] Diff viewer component
- [x] Inline/side-by-side toggle
- [x] File click -> diff flow

## Phase 6: FS Watcher
- [x] Tauri event emission (git/watcher.rs)
- [x] Frontend auto-refresh hook (use-tauri-event.ts + use-git-status.ts)

## Phase 7: Branch Management
- [x] Status bar branch display
- [x] Branch picker with search

## Phase 8: Remote Operations
- [x] Push/pull/fetch
- [x] Sync button (action-button.tsx)
- [x] Context-aware action button

## Phase 9: Polish
- [x] Tree view mode
- [x] Open Repository dialog (Ctrl+O with tauri-plugin-dialog)
- [x] Keyboard shortcuts (Ctrl+O open, Ctrl+Enter commit, Ctrl+Shift+G refresh, Ctrl+Shift+P toggle diff mode)
- [x] Context menus (right-click on resource items)
- [x] Error toasts
- [x] Remember last repo (localStorage)
- [x] Window title (shows repo name + branch via Tauri set_title)
