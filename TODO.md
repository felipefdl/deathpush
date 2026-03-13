# DeathPush Native (Swift) -- TODO

Feature parity tracker between the Tauri version and the native macOS rewrite.

## Legend

- [x] Complete
- [~] Partial (backend exists, UI missing or incomplete)
- [ ] Missing

---

## Core Git Operations

- [x] Open repository (directory picker, CLI arg, deep link)
- [x] Clone repository (URL + path dialog)
- [x] Init repository (git init)
- [x] File system watcher (Rust-side, debounced refresh)
- [x] Git status with resource groups (Index, Working Tree, Untracked, Merge)
- [x] Stage / unstage individual files
- [x] Stage / unstage all files
- [x] Hunk-level staging and discard
- [x] Line-level staging (partial hunk)
- [x] Discard changes (with confirmation)
- [x] Commit with message
- [x] Amend commit
- [x] Undo last commit (soft reset HEAD~1)
- [x] Commit + Push / Commit + Sync workflows
- [x] Branch list (local + remote, ahead/behind)
- [x] Checkout / switch branch
- [x] Create branch (with optional base)
- [x] Delete branch (local, with force option)
- [x] Rename branch
- [x] Delete remote branch
- [x] Fetch (with prune)
- [x] Pull (with rebase option)
- [x] Push (with force-with-lease option)
- [x] Merge branch + continue / abort
- [x] Rebase branch + continue / abort / skip
- [x] Cherry-pick commit (from history context menu)
- [x] Reset to commit (soft / mixed / hard)
- [x] Operation state detection (merge / rebase / cherry-pick / revert)
- [x] Stash save (with message)
- [x] Stash save include untracked
- [x] Stash save staged only
- [x] Stash list / apply / pop / drop
- [x] Stash show (diff view)
- [x] Tag list (local + remote, annotated flag)
- [x] Create tag (lightweight or annotated)
- [x] Delete tag (local + remote)
- [x] Push tag to remote
- [x] Git blame (full view with color-coded groups)
- [x] File history (per-file log with --follow)
- [x] Last commit info in status bar
- [x] Git config read/write (user.name, user.email)
- [x] Git output panel (command log with duration)
- [ ] Revert commit (git revert <sha>)
- [ ] Inline blame in status bar (current cursor line annotation)

## Diff Viewer

- [x] Monaco-based diff in WKWebView
- [x] Side-by-side diff mode
- [x] Inline diff mode
- [x] Image diff (side-by-side with metadata)
- [x] Language detection / syntax highlighting
- [x] Editor settings applied (font, tab size, word wrap, whitespace)
- [x] Theme-aware (color theme applied to Monaco)

## File Explorer

- [x] Directory tree with lazy loading
- [x] Gitignore-aware filtering
- [x] Git status decoration (colors)
- [x] File icon themes (Material icons)
- [x] File viewer (Monaco read-only editor)
- [x] Image viewer
- [x] Binary file detection
- [x] Context menu (new file/folder, rename, delete, duplicate, reveal, copy path, add to .gitignore)
- [x] Inline creation (new file/folder)
- [x] File filter / search
- [ ] Copy / Cut / Paste files (Cmd+C/X/V)
- [ ] Import files from outside repo
- [ ] Drag and drop file operations
- [ ] Explorer file editing and saving (currently read-only)

## Quick Open (Cmd+P)

- [x] Fuzzy file search (nucleo)
- [x] Content search (#query mode, git grep)
- [x] Go to line (file:line syntax)
- [x] Keyboard navigation
- [ ] Recent files shown when search is empty

## Terminal

- [x] Full terminal emulator (SwiftTerm, native -- upgrade over xterm.js)
- [x] Multiple terminal tabs/groups
- [x] Split panes (horizontal + vertical, 4 directions)
- [x] Terminal search / find
- [x] Kill terminal session
- [x] Foreground process detection (tab title)
- [x] Shell environment resolution (full PATH)
- [x] Process completion notifications (>5s commands)
- [x] Bell notifications (native macOS)
- [x] OSC 9 desktop notification support
- [x] Theme-aware ANSI colors
- [x] Configurable: cursor style, blink, option-as-meta, mouse reporting, scrollback, bold-as-bright
- [x] Terminal toggle (Cmd+J)
- [ ] Custom shell path / shell presets in settings (zsh, bash, fish, etc.)
- [ ] Terminal maximize mode (full-screen terminal)

## Themes

- [x] 30+ VS Code-compatible color themes
- [x] Include chain resolution
- [x] Theme picker with search + live preview (Cmd+Shift+T)
- [x] Dark/light/high-contrast support
- [x] Terminal colors extracted from theme
- [ ] Preferred dark/light theme (auto-switch on system appearance change)
- [ ] Icon theme picker (separate picker UI, Cmd+K Cmd+I in Tauri)

## Settings

- [x] Appearance tab (color theme)
- [x] Editor tab (font, size, line height, tab size, word wrap, whitespace, diff mode)
- [x] Terminal tab (font, cursor, blink, option-as-meta, mouse, scrollback, bold-as-bright, bell, process notifications)
- [x] Git tab (user.name, user.email, auto-fetch toggle + interval, confirm-before-discard)
- [ ] Projects tab (workspace directory management -- exists in Welcome screen but not in Settings)
- [ ] Sidebar position (left/right toggle)

## UI / Layout

- [x] Sidebar with SCM + Explorer tabs
- [x] Resizable sidebar
- [x] Commit input with amend / push / sync
- [x] Branch picker (status bar + sheets)
- [x] Merge/rebase banner with action buttons
- [x] Error toast notifications (auto-dismiss)
- [x] Status bar (branch, ahead/behind, last commit)
- [x] Toolbar pill (branch + operation state)
- [x] Repo switcher popovers (recents + workspace)
- [x] Empty state views
- [x] SCM list view + tree view toggle
- [x] File filter in SCM
- [x] Collapsible resource group panes
- [x] Nested repositories section
- [x] Stash section in SCM
- [x] Tags section in SCM
- [ ] Multi-file selection in SCM (Ctrl/Cmd-click)
- [ ] Keyboard navigation in SCM (Arrow keys, Space to stage/unstage, Enter to open diff)
- [ ] SCM overflow menu (...) with additional actions
- [ ] Window close confirmation for uncommitted changes (WindowCloseGuard exists but verify)

## History View

- [x] Paginated commit list
- [x] Commit detail (author, date, SHA, message, changed files)
- [x] Per-file diff in commit detail
- [x] Gravatar avatars (with NSCache)
- [x] Search/filter commits
- [x] Context menu (copy SHA, cherry-pick, reset)
- [x] Resizable commit list pane

## Platform / App

- [x] Native macOS tabs (Cmd+T)
- [x] Session-based architecture (TabRegistry)
- [x] CLI tool installation (dp / deathpush symlinks)
- [x] Deep link URL scheme (deathpush://)
- [x] Welcome screen with recent + workspace projects
- [x] Workspace directory scanning with configurable depth
- [x] Clone dialog
- [x] Native macOS notifications
- [x] Auto-fetch on configurable interval
- [~] Auto-update (Sparkle stub exists, not integrated)
- [~] Worktree detection (backend exists, no UI)
- [ ] Open Source Licenses modal (Help > Licenses)
- [ ] Native menu: full File/Edit/View/Git/Terminal/Help structure (partially done via .commands)

## Not Applicable (Tauri-specific)

These features from the Tauri version are not needed or are replaced by native equivalents:

- Linux/Windows support (native macOS only)
- Custom title bar (uses native macOS title bar)
- Zoom level (Cmd+=/Cmd+-) -- not applicable for native views
- xterm.js terminal -- replaced by SwiftTerm (upgrade)
- localStorage persistence -- replaced by @AppStorage / UserDefaults
- Tauri webview events -- replaced by UniFFI callbacks + Combine

## Native-Only Features (not in Tauri)

Features the Swift version has that Tauri doesn't:

- [x] Native macOS tabs (NSWindow tabbing)
- [x] SwiftTerm native terminal (better performance than xterm.js)
- [x] OSC 9 terminal notifications
- [x] Process completion notifications
- [x] Commit + Push / Commit + Sync one-click workflows
- [x] Confirm before discard setting
- [x] Auto-fetch with configurable interval
- [x] Repo switcher toolbar popovers
