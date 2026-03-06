# DeathPush - Agent Instructions

## Project Overview

DeathPush is a standalone desktop Git client built with Tauri v2 (Rust backend, React + TypeScript frontend) that replicates the VS Code Source Control UX. It provides staging, committing, diffing, branch management, push/pull, stash, tags, commit history, hunk staging, merge/rebase detection, clone, cherry-pick, reset, git blame, integrated terminal, settings, multi-window support, and a welcome/project picker screen in a lightweight native app.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Frontend | React 19 + TypeScript |
| Backend | Rust (Tauri v2) |
| State | Zustand |
| Diff viewer | Monaco Editor (@monaco-editor/react) |
| Terminal | xterm.js + portable-pty (Rust) |
| Icons | VS Code Codicon font (@vscode/codicons) |
| Package manager | npm |
| Linter | oxlint |
| Formatter | oxfmt (TS), rustfmt (Rust) |
| Tests | Vitest (jsdom) |
| Task runner | justfile |

## Architecture

### Git Strategy: Hybrid

- **Read ops** (status, diff, branches, log, tags, ahead/behind): git2 crate for speed (no process spawn)
- **Write ops** (add, commit, push, pull, checkout, fetch, stash, cherry-pick, reset, clone): git CLI via `tokio::process::Command` for hook execution, GPG signing, credential helpers, SSH config, and LFS support
- **Blame/file-log ops**: git CLI via `tokio::process::Command` (porcelain blame, follow log)

### Multi-Window

- Each window has its own `RepoState` (git2 repo handle + CLI root path) stored in `AppRepoState` keyed by window label
- Each window has its own FS watcher (`WatcherState`) and terminal sessions (`TerminalState`)
- Window cleanup on destroy removes repo state, watcher, and terminal sessions
- CLI argument support: `deathpush /path/to/repo` opens directly

### Backend (src-tauri/src/)

- `error.rs` -- Error type via thiserror with Serialize impl
- `types.rs` -- Serde DTOs shared with frontend (FileStatus, ResourceGroup, RepositoryStatus, DiffContent, BranchEntry, CommitEntry, CommitDetail, StashEntry, TagEntry, DiffHunk, FileDiffWithHunks, RepoOperationState, BlameLineGroup, FileBlame, LastCommitInfo, ProjectInfo)
- `pty.rs` -- PTY session management via portable-pty (spawn shell, read/write, resize, per-window sessions)
- `git/repository.rs` -- git2::Repository wrapper (open, head, ahead/behind)
- `git/status.rs` -- git2 status flags -> resource groups + operation state detection
- `git/diff.rs` -- Blob reads via git2 for Monaco diff (HEAD, index, working tree)
- `git/branch.rs` -- Branch listing via git2 with ahead/behind counts
- `git/log.rs` -- Commit history via git2 revwalk (sorted by time)
- `git/tag.rs` -- Tag listing via git2
- `git/hunk.rs` -- Parse unified diff into hunks, generate partial patches
- `git/repo_state.rs` -- Detect merge/rebase/cherry-pick/revert in progress via `.git/` sentinels
- `git/blame.rs` -- Git blame (porcelain), file log (--follow), last commit info via CLI
- `git/cli.rs` -- Async git CLI runner for all write ops
- `git/watcher.rs` -- FS watcher (notify + debouncer, 500ms) emitting Tauri events, per-window
- `commands/` -- Thin Tauri command handlers (repository, status, staging, commit, branch, remote, log, stash, tag, file_ops, lifecycle, terminal, blame, config)
- `lib.rs` -- App builder with managed state (AppRepoState, TerminalState, WatcherState), native menu system, multi-window support, 58 commands registered

### Frontend (src/)

- `stores/repository-store.ts` -- Zustand store (status, files, diff, branches, stashes, tags, commitLog, operations, fileFilter, focusedIndex, amendMode, fileHunks, terminalGroups)
- `stores/layout-store.ts` -- Zustand store for layout (sidebarWidth, terminalVisible, terminalHeight, mainView, diffMode, viewMode, panelTab, collapsedPanes, terminalMaximized) with per-project localStorage persistence
- `stores/theme-store.ts` -- Zustand store for color theme (currentTheme, setTheme)
- `stores/icon-theme-store.ts` -- Zustand store for file icon theme (currentIconTheme, setIconTheme)
- `stores/settings-store.ts` -- Zustand store for app settings (UI, editor, terminal, git, projects) with localStorage persistence
- `lib/tauri-commands.ts` -- Typed invoke() wrappers for all 58 Tauri commands
- `lib/git-types.ts` -- TypeScript types matching Rust DTOs (including BlameLineGroup, FileBlame, LastCommitInfo)
- `lib/flat-file-list.ts` -- Flatten resource groups into indexed file list for keyboard nav
- `lib/format-date.ts` -- Relative date formatting
- `lib/status-colors.ts` -- FileStatus -> CSS variable color
- `lib/status-icons.ts` -- FileStatus -> single letter label
- `lib/constants.ts` -- App constants (APP_NAME, DEFAULT_REMOTE)
- `lib/recent-projects.ts` -- Recent project history (localStorage, max 20)
- `lib/toggle-terminal.ts` -- Terminal toggle logic
- `lib/themes/` -- Color theme infrastructure (types, registry, defaults, apply-theme, json/)
- `lib/icon-themes/` -- File icon theme infrastructure (types, registry, apply, get-icon-classes, generate-icon-css)
- `hooks/` -- use-repository, use-git-status, use-diff, use-branches, use-keyboard-shortcuts, use-tauri-event, use-commit-log, use-stash, use-tags, use-resize-observer
- `components/scm/` -- SCM view, commit input (with amend/undo), resource groups (list + tree), resource item, file filter, stash view, action button, context menu, merge banner, overflow menu, SCM toolbar, resizable pane container
- `components/diff/` -- Monaco DiffEditor with inline/side-by-side + hunk view, diff header, image diff, empty state
- `components/history/` -- Commit history (commit-list with cherry-pick/reset context menu, commit-detail, history-view)
- `components/branch/` -- Branch picker with search, create, branch item, and tags section (tag-item)
- `components/terminal/` -- Terminal panel, terminal instance (xterm.js), terminal group view, git output panel
- `components/layout/` -- App layout, main panel (Changes/History/Settings tabs), status bar, title bar (macOS overlay), clone dialog
- `components/settings/` -- Settings page (UI, editor, terminal, git, projects configuration)
- `components/welcome/` -- Welcome screen with recent projects and project directory scanner
- `components/theme/` -- Theme picker, icon theme picker (VS Code command palette style)
- `components/ui/` -- Spinner
- `styles/global.css` -- Base styles + theme picker CSS (no hardcoded colors; all set by JS via applyTheme)
- `styles/scm.css` -- SCM, merge banner, clone dialog, stash, filter, keyboard focus styles
- `styles/history.css` -- Commit history styles
- `styles/terminal.css` -- Terminal panel styles
- `styles/welcome.css` -- Welcome screen styles
- `styles/settings.css` -- Settings page styles
- `styles/codicons.css` -- VS Code Codicon font styles

### Tauri Commands (API Surface - 58 total)

| Command | Returns | Method |
|---------|---------|--------|
| `open_repository(path)` | RepositoryStatus | git2 + watcher |
| `get_initial_path()` | String? | CLI args |
| `scan_projects_directory(path, depth)` | Vec\<ProjectInfo\> | filesystem |
| `get_status()` | RepositoryStatus | git2 |
| `get_file_diff(path, staged)` | DiffContent | git2 |
| `stage_files(paths)` | RepositoryStatus | CLI |
| `stage_all()` | RepositoryStatus | CLI |
| `unstage_files(paths)` | RepositoryStatus | CLI |
| `unstage_all()` | RepositoryStatus | CLI |
| `discard_changes(paths)` | RepositoryStatus | CLI |
| `commit(message, amend)` | RepositoryStatus | CLI |
| `list_branches()` | Vec\<BranchEntry\> | git2 |
| `checkout_branch(name)` | RepositoryStatus | CLI |
| `create_branch(name, from)` | RepositoryStatus | CLI |
| `delete_branch(name, force)` | () | CLI |
| `push(remote, branch, force)` | () | CLI |
| `pull(remote, branch, rebase)` | () | CLI |
| `fetch(remote, prune)` | () | CLI |
| `get_commit_log(skip, limit)` | Vec\<CommitEntry\> | git2 |
| `get_commit_detail(id)` | CommitDetail | git2 |
| `get_commit_file_diff(commit_id, path)` | CommitDiffContent | git2 |
| `get_last_commit_message()` | String | CLI |
| `undo_last_commit()` | RepositoryStatus | CLI |
| `stash_save(message?)` | RepositoryStatus | CLI |
| `stash_list()` | Vec\<StashEntry\> | CLI |
| `stash_apply(index)` | RepositoryStatus | CLI |
| `stash_pop(index)` | RepositoryStatus | CLI |
| `stash_drop(index)` | Vec\<StashEntry\> | CLI |
| `list_tags()` | Vec\<TagEntry\> | git2 |
| `create_tag(name, message?, commit?)` | Vec\<TagEntry\> | CLI |
| `delete_tag(name)` | Vec\<TagEntry\> | CLI |
| `push_tag(remote, tag)` | () | CLI |
| `open_in_editor(path)` | () | platform |
| `reveal_in_file_manager(path)` | () | platform |
| `add_to_gitignore(pattern)` | RepositoryStatus | filesystem |
| `write_file(path, content)` | () | filesystem |
| `delete_file(path)` | RepositoryStatus | filesystem |
| `get_file_hunks(path, staged)` | FileDiffWithHunks | CLI + parse |
| `stage_hunk(path, hunk_index, staged)` | RepositoryStatus | CLI apply |
| `clone_repository(url, path)` | RepositoryStatus | CLI |
| `merge_continue()` | RepositoryStatus | CLI |
| `merge_abort()` | RepositoryStatus | CLI |
| `rebase_continue()` | RepositoryStatus | CLI |
| `rebase_abort()` | RepositoryStatus | CLI |
| `rebase_skip()` | RepositoryStatus | CLI |
| `cherry_pick(commit_id)` | RepositoryStatus | CLI |
| `reset_to_commit(id, mode)` | RepositoryStatus | CLI |
| `terminal_spawn(cols, rows)` | SpawnResult | PTY |
| `terminal_write(id, data)` | () | PTY |
| `terminal_resize(id, cols, rows)` | () | PTY |
| `terminal_kill(id)` | () | PTY |
| `terminal_foreground_process(id)` | String | process |
| `get_git_config(key)` | String | CLI |
| `set_git_config(key, value)` | () | CLI |
| `get_file_blame(path)` | FileBlame | CLI |
| `get_file_log(path, skip, limit)` | Vec\<CommitEntry\> | CLI |
| `get_last_commit_info()` | LastCommitInfo | CLI |
| `new_window()` | () | Tauri |

### Tauri Events

- `repository-changed` -- FS watcher -> frontend auto-refresh (per-window)
- `terminal:data` -- PTY output -> frontend (per-session, includes id)
- `terminal:exit` -- Terminal session exited (per-session)
- `menu:*` -- Native menu events forwarded to frontend (e.g. `menu:preferences`, `menu:open-repo`, `menu:toggle-terminal`)

### Native Menu

DeathPush, File (New Window, Open Repo, Clone), Edit, View (Changes, History, Toggle Diff Mode), Git (Pull, Push, Fetch, Stage/Unstage All, Stash, Undo Commit), Terminal (New, Kill, Toggle), Window, Help.

## Conventions

### Rust

- Edition 2024, minimum 1.85.0
- Run clippy with `-D warnings`
- rustfmt config in `rustfmt.toml`: `max_width = 120`, `tab_spaces = 2`
- Use `thiserror` for error types
- Use `tracing` for logging (not `println!` or `log`)
- Async with tokio for CLI operations
- All DTOs use `#[serde(rename_all = "camelCase")]`
- Write ops return updated `RepositoryStatus` to keep frontend in sync

### TypeScript

- Strict mode, no `any`
- Double quotes, semicolons always, trailing commas ES5
- Line width: 120 characters
- `const` over `let`, never `var`
- camelCase for functions/variables, PascalCase for types/components
- SCREAMING_SNAKE_CASE for constants
- kebab-case for files and directories
- Named exports only (no default exports)

### File Organization

- `src-tauri/src/commands/` -- Tauri command handlers (thin, delegate to git/ or pty)
- `src-tauri/src/git/` -- Git operations (git2 reads, CLI writes, blame)
- `src-tauri/src/pty.rs` -- PTY session management (portable-pty)
- `src/components/` -- React components organized by feature (scm/, diff/, branch/, history/, terminal/, layout/, settings/, welcome/, theme/, ui/)
- `src/hooks/` -- Custom React hooks
- `src/stores/` -- Zustand stores (repository, layout, theme, icon-theme, settings)
- `src/lib/` -- Utilities, types, constants
- `src/lib/themes/` -- Color theme infrastructure
- `src/lib/icon-themes/` -- File icon theme infrastructure
- `src/styles/` -- CSS (global.css, scm.css, history.css, terminal.css, welcome.css, settings.css, codicons.css)

### Git Operations Pattern

- Read ops (status, diff, branches, log, tags): use git2 crate directly for speed
- Write ops (add, commit, push, pull, checkout, stash, etc.): spawn git CLI via tokio::process::Command
- Blame/file-log: spawn git CLI (porcelain blame, follow log)
- Always reopen git2::Repository and refresh status after write operations
- Use Mutex<AppRepoState> managed state to share per-window repo handles across commands

### Testing

- Vitest with jsdom environment
- TZ=UTC for all tests
- Test files: `src/**/*.test.{ts,tsx}`
- Exclude `.temp-vscode/` from test discovery

### Discard Operations

- Always show a native confirm dialog before discarding changes (destructive, irreversible)
- Uses `confirm()` from `@tauri-apps/plugin-dialog`

### Themes

- Theme system uses VS Code's native JSON format (`src/lib/themes/json/`)
- All bundled themes come from VS Code built-in extensions or MIT-licensed community projects
- When adding new themes, always verify the source license permits redistribution (MIT, Apache-2.0, ISC, BSD, or similar permissive license) -- never bundle themes with restrictive or unclear licenses
- VS Code built-in themes ship under the VS Code MIT license and can always be used
- CSS variables are set dynamically by `applyTheme()` at startup (no hardcoded colors in `:root`)
- Color key conversion: `editor.background` -> `--vscode-editor-background` (dots become hyphens, prefix `--vscode-`)
- Monaco themes registered via `defineTheme()` with `tokenColors` from the JSON
- Terminal theme extracted from resolved theme `colors` at runtime via `getTerminalTheme()`
- Theme picker opens via Cmd+K Cmd+T chord or status bar icon

### Icon Themes

- File icon themes use VS Code icon theme JSON format (`src/lib/icon-themes/`)
- Icon theme registry resolves theme definitions into CSS classes
- `applyIconTheme()` generates and injects CSS for file/folder icons
- Icon theme picker available alongside color theme picker
- Icon theme persisted in localStorage

### Settings

- App settings stored in localStorage under `deathpush:settings`
- Sections: UI (font, sidebar position), Editor (font, tab size, word wrap, minimap, whitespace), Terminal (font, cursor), Git (blame toggle), Projects (directory, scan depth)
- Settings page accessible via Cmd+, or DeathPush menu

### Layout Persistence

- Layout state (sidebar width, terminal visibility/height, diff mode, view mode, panel tab, collapsed panes) persisted per-project in localStorage
- Key format: `deathpush:layout:{base64(root)}`
- Transient views (settings, terminal, output) reset to "changes" on reload

## VS Code Reference

The `.temp-vscode/` directory contains VS Code source for reference. Key files:
- `extensions/git/src/repository.ts` -- Status classification logic (lines 2914-2964)
- `extensions/git/src/git.ts` -- Git CLI wrapper model
- `extensions/git/src/commands.ts` -- All git commands
- `src/vs/workbench/contrib/scm/browser/scmViewPane.ts` -- SCM tree rendering
- `src/vs/workbench/contrib/scm/browser/media/scm.css` -- SCM styles

## Development

```sh
just dev          # Start Tauri dev server
just build        # Production build
just lint         # Run oxlint + clippy
just fmt          # Format with oxfmt + rustfmt
just check        # Type check (tsc + cargo check)
just test         # Run vitest
just test-watch   # Run vitest in watch mode
```
