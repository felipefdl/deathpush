# DeathPush - Agent Instructions

## Project Overview

DeathPush is a standalone desktop Git client built with Tauri v2 (Rust backend, Solid 2 + TypeScript frontend) that replicates the VS Code Source Control UX. It provides staging, committing, diffing, branch management, push/pull, stash, tags, commit history, hunk staging, merge/rebase detection, clone, cherry-pick, reset, git blame, integrated terminal, settings, multi-window support, CLI tool (`dp`/`deathpush`), auto-update, and a welcome/project picker screen in a lightweight native app.

## Brand & Voice

- DeathPush (or "dp" for short) is a dark humor brand name, intentionally edgy -- similar to Liquid Death. It is not offensive, it is branding.
- Dark humor in copy, UI text, and documentation is intentional and must be preserved as-is.
- Agents must NOT sanitize, tone-police, soften, or rewrite text for "marketing purposes" or politeness.
- This is a git client, not a sensitivity project. Respect the developer audience.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| Frontend | Solid 2 + TypeScript |
| Backend | Rust (Tauri v2) |
| State | Zustand vanilla stores with Solid selectors |
| Diff viewer | Pierre FileDiff + Editor |
| Terminal | WTerm DOM + Ghostty core + portable-pty (Rust) |
| Icons and trees | `@pierre/trees` in Explorer/SCM; VS Code Codicons elsewhere |
| Package manager | Vite+ (`vp`) / pnpm |
| Linter | `vp lint` |
| Formatter | `vp fmt` (TS), rustfmt (Rust) |
| Tests | `vp test` (jsdom) |
| Task runner | justfile |

## Architecture

### Git Strategy: Hybrid

- **Read ops** (status, SCM diffs, branches, log, tags, ahead/behind): git2. `scm_file_diff` builds original/modified/hunks from one git2 diff. SCM hunks do not spawn `git diff`.
- **Write ops** (add, commit, push, pull, checkout, fetch, stash, cherry-pick, reset, clone, `git apply` patches): git CLI via `tokio::process::Command` for hooks, GPG, credentials, SSH, and LFS
- **Blame/file-log ops**: git CLI via `tokio::process::Command` (porcelain blame, follow log)

### Multi-Window

- Each window has `RepoState` in `AppRepoState` and a `SessionRegistry` entry, keyed by window label
- `session_intent` takes a per-window tokio intent lock, then mutates live `SessionState` through a generation-bound `SessionHandle`. Production never clones `SessionState` across `.await`. A reset cannot be overwritten by a later replace.
- Watcher `status_event` uses the registry mutex only and does not take the intent lock. Two windows on one root have two locks.
- Windows on the same canonical root share one `RepositoryRuntime` (status coordinator, FS watcher, Quick Open `FileIndex`)
- Terminal sessions stay per-window (`TerminalState`)
- Destroy unbinds the window from the runtime and drops that window's session, intent lock, repo state, and PTYs
- CLI argument support: `deathpush /path/to/repo` opens directly

### Backend (src-tauri/src/)

- `error.rs` -- Error type via thiserror with Serialize impl
- `types.rs` -- Shared Serde DTOs (status, diffs, explorer, paths-changed)
- `session/` -- Intent apply (`RefreshImpact`), `SessionHandle`, `SessionSnapshot` / `SessionStatusEvent` (two cursor pairs)
- `content_hash.rs` -- SHA-256 of Pierre-compared UTF-8 strings
- `pty.rs` -- PTY session management via portable-pty (spawn shell, read/write, resize, per-window sessions)
- `git/repository.rs` -- git2::Repository wrapper (open, head, ahead/behind)
- `git/status.rs` -- git2 status flags -> resource groups + operation state detection
- `git/diff.rs` -- `scm_file_diff` via git2 for Pierre FileDiff (HEAD, index, working tree, hunks)
- `git/branch.rs` -- Branch listing via git2 with ahead/behind counts
- `git/log.rs` -- Commit history via git2 revwalk (sorted by time)
- `git/tag.rs` -- Tag listing via git2
- `git/hunk.rs` -- Hunk ids and patches from live hunks (no raw unified-patch cache)
- `git/invalidation.rs` -- `classify_git_relative` shared by watcher and intent (`packed-refs`, `refs/stash`, `logs/refs/stash`)
- `git/repo_state.rs` -- Detect merge/rebase/cherry-pick/revert in progress via `.git/` sentinels
- `git/blame.rs` -- Git blame (porcelain), file log (--follow), last commit info via CLI
- `git/cli.rs` -- Async git CLI runner for write ops
- `git/watcher.rs` -- notify watcher feeding the shared runtime (one per canonical root)
- `git/repository_runtime.rs` -- Per-root runtime: status, watcher, `FileIndex`, `invalidate_refs` / `invalidate_stashes`, window fan-out
- `git/status_coordinator.rs` -- Coalesced status scans, git invalidation, `repository:paths-changed`
- `commands/` -- Thin handlers: repository, session, explorer, file_ops, terminal, config, cli
- `lib.rs` -- App builder, native menu, multi-window; managed `AppRepoState`, `TerminalState`, `RepositoryRuntimeRegistry`, `SessionRegistry`

### Frontend (src/)

- `stores/repository-store.ts` -- Zustand store (status, files, diff, branches, stashes, tags, commitLog, operations, fileFilter, amendMode, fileHunks, terminalGroups)
- `stores/layout-store.ts` -- Zustand store for layout (sidebarWidth, terminalVisible, terminalHeight, mainView, panelTab, collapsedPanes, terminalMaximized) with per-project localStorage persistence
- `stores/theme-store.ts` -- Zustand store for color theme (currentTheme, setTheme)
- `stores/settings-store.ts` -- Zustand store for app settings (UI, editor, diff viewer, terminal, git, projects) with localStorage persistence, including tree density and icon presets
- `lib/tauri-commands.ts` -- Typed invoke() wrappers for non-session commands
- `lib/session-client.ts` -- `sessionIntent` / `getSessionSnapshot`. Repo/groups follow the status cursor; session-derived fields follow the session cursor. Escape/deselect sends `ClearFile`. Late Diff/Blame from an old generation/root is ignored.
- `lib/git-types.ts` -- TypeScript types matching Rust DTOs (`Intent`, `IntentOutcome`, `SessionSnapshot`)
- `lib/format-date.ts` -- Relative date formatting
- `lib/status-colors.ts` -- FileStatus -> CSS variable color
- `lib/status-icons.ts` -- FileStatus -> single letter label
- `lib/constants.ts` -- App constants (APP_NAME, DEFAULT_REMOTE)
- `lib/recent-projects.ts` -- Recent project history (localStorage, max 20)
- `lib/toggle-terminal.ts` -- Terminal toggle logic
- `lib/workspace-tree.ts` -- Build tree structure from scanned projects for welcome screen
- `lib/author-utils.ts` -- Author initials extraction + deterministic avatar color hashing
- `lib/pierre/` -- Pierre worker pool, theme register, options, keymap, find host, flush, save session, hunk annotations, line map
- `lib/updater.ts` -- Tauri auto-update check + download wrapper
- `lib/themes/` -- Color theme infrastructure (types, registry, apply-theme)
- `lib/trees.ts` -- Adapters from DeathPush repository data to Pierre Trees paths and Git status
- `hooks/` -- use-repository, use-git-status, use-diff, use-branches, use-keyboard-shortcuts, use-tauri-event, use-commit-log, use-stash, use-tags, use-resize-observer, use-color-scheme
- `components/scm/` -- SCM view, commit input (with amend/undo), Pierre Trees resource groups, file filter, stash view, action button, context menu, merge banner, overflow menu, SCM toolbar, resizable pane container
- `components/trees/` -- Solid lifecycle host and context-menu bridge for `@pierre/trees`
- `components/pierre/` -- VirtualizedFile + Editor (FileViewer), FileDiff + Editor (SCM and history), UnresolvedFile (merge)
- `components/diff/` -- Diff header, image/binary/large panes, empty state around Pierre hosts
- `components/history/` -- Commit history (commit-list with cherry-pick/reset context menu, commit-detail, commit-file-tree, history-view)
- `components/branch/` -- Branch picker with search, create, branch item, and tags section (tag-item)
- `components/terminal/` -- Terminal panel, WTerm/Ghostty terminal instance, terminal group view, git output panel
- `components/layout/` -- App layout, main panel (Changes/History/Settings tabs), status bar, title bar (macOS overlay), clone dialog
- `components/settings/` -- Settings page (UI, editor, diff viewer, terminal, git, projects configuration)
- `components/welcome/` -- Welcome screen with recent projects and project directory scanner
- `components/theme/` -- Color theme picker (VS Code command palette style)
- `components/shared/` -- Workspace config modal (multi-workspace directory configuration)
- `components/ui/` -- Spinner
- `styles/global.css` -- Base styles + theme picker CSS (no hardcoded colors; all set by JS via applyTheme)
- `styles/scm.css` -- SCM, merge banner, clone dialog, stash, filter, keyboard focus styles
- `styles/history.css` -- Commit history styles
- `styles/terminal.css` -- Terminal panel styles
- `styles/welcome.css` -- Welcome screen styles
- `styles/settings.css` -- Settings page styles
- `styles/codicons.css` -- VS Code Codicon font styles

### Tauri Commands

Git status, diffs, blame, branches, log, stash, tags, hunks, and all git writes go through `session_intent`. There are no `open_repository`, `get_status`, `get_file_diff`, or per-op git write commands.

`session_intent` outcomes: `ack` (optional `sessionGeneration`/`sessionRevision`), `patch` / `diff` / `blame` (stamped), `needsConfirmation` (no bump, no stamp), `snapshot` (session + status cursor pairs). Status-only, refs, and stash refreshes return stamped Ack. Open, clone, refresh, and HEAD-moving writes return one Snapshot. Snapshots are command results, not a `session:snapshot` emit.

| Command | Returns |
|---------|---------|
| `session_intent(intent)` | IntentOutcome (`ack`, `patch`, `snapshot`, `diff`, `blame`, `needsConfirmation`) |
| `get_session_snapshot()` | SessionSnapshot |
| `get_initial_path()` | String? |
| `scan_workspace_projects(entries)` | Vec\<ProjectInfo\> (`entries`: `{ directory, depth }[]`) |
| `discover_nested_repositories()` | Vec\<NestedRepository\> (`path`, `name`, `branch` nullable) |
| `detect_worktrees()` | Vec\<WorktreeInfo\> |
| `list_repository_tree()` | Vec\<ExplorerEntry\> |
| `list_repository_children(path)` | Vec\<ExplorerEntry\> |
| `read_file_content(path)` | FileContent (includes `contentHash`) |
| `fuzzy_find_files(query, maxResults)` | Vec\<FuzzyFileResult\> (cached `FileIndex`, not `git ls-files` per query) |
| `search_file_contents(query, maxResults)` | Vec\<ContentSearchResult\> (live `git grep`) |
| `write_file(path, content)` | WriteFileResult `{ contentHash }` |
| `open_in_editor(path)` | () |
| `reveal_in_file_manager(path)` | () |
| `rename_entry(oldPath, newName)` | () |
| `create_directory(path)` | () |
| `copy_entries(sources, destinationDir, onConflict?)` | () |
| `move_entries(sources, destinationDir, onConflict?)` | () |
| `duplicate_entry(path)` | String |
| `import_files(sources, destinationDir, onConflict?)` | () |
| `terminal_spawn(cols, rows, shellPath?)` | SpawnResult |
| `terminal_write(id, data)` | () |
| `terminal_resize(id, cols, rows)` | () |
| `terminal_kill(id)` | () |
| `terminal_foreground_process(id)` | String |
| `terminals_have_active_process()` | bool |
| `get_git_config(key)` | String |
| `set_git_config(key, value)` | () |
| `check_cli_installed()` | CliInstallStatus |
| `install_cli()` | () |
| `uninstall_cli()` | () |
| `new_window(path?)` | () |
| `set_repo_menu_enabled(enabled)` | () |
| `set_native_theme(dark)` | () |
| `quit_app()` | () |
| `window_minimize()` | () |
| `window_maximize()` | () |
| `window_close()` | () |
| `window_confirm_close()` | () |

### Tauri Events

- `session:status` -- per-window status from the shared runtime: `sessionGeneration`/`sessionRevision`, `statusGeneration`/`statusRevision`, groups, optional `extras` (lastCommit, branches, tags, commitLog, stashes)
- `repository:paths-changed` -- FS path changes from the watcher (same payload to every window on that root)
- `watcher:error` -- watcher failed to start
- `git:command` -- git CLI invocation log
- `terminal:data` -- PTY output (per-session, includes id)
- `terminal:exit` -- terminal session exited (per-session)
- `window:close-requested` -- close intercepted until the frontend confirms
- `menu:*` -- native menu events (e.g. `menu:preferences`, `menu:open-repo`, `menu:toggle-terminal`)

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
- Session git writes return `IntentOutcome` from `session_intent`, not a refreshed `RepositoryStatus`

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

- `src-tauri/src/commands/` -- Tauri command handlers (repository, session, explorer, file_ops, terminal, config, cli)
- `src-tauri/src/session/` -- Intent apply (`SessionHandle`, `RefreshImpact`) and session snapshot
- `src-tauri/src/git/` -- Git operations (git2 reads including `scm_file_diff`, CLI writes, invalidation, FileIndex, per-root runtime/watcher)
- `src/components/` -- Solid components organized by feature (scm/, explorer/, trees/, diff/, pierre/, branch/, history/, terminal/, layout/, settings/, welcome/, theme/, shared/, ui/)
- `src/hooks/` -- Custom Solid reactive utilities
- `src/stores/` -- Zustand stores (repository, layout, theme, settings)
- `src/lib/` -- Utilities, types, constants, Trees adapters, Pierre hosts (`lib/pierre/`)
- `src/lib/themes/` -- Color theme infrastructure
- `src/styles/` -- CSS (global.css, scm.css, history.css, terminal.css, welcome.css, settings.css, codicons.css)

### Git Operations Pattern

- Read ops (status, SCM diffs, branches, log, tags): git2 crate directly
- Write ops (add, commit, push, pull, checkout, stash, clone, ...): git CLI via `session_intent`
- Blame/file-log: git CLI (porcelain blame, follow log)
- `apply_intent` classifies git writes as `RefreshImpact` (`StatusPaths`, `StatusRepository`, `Refs`, `Stashes`, `StatusAndStashes`, `Snapshot`). Status-only paths invalidate those paths (no force-baseline, no extras, no Snapshot). Refs/stashes call `invalidate_refs` / `invalidate_stashes`. Open/clone/refresh and HEAD-moving writes return one Snapshot.
- Watcher and intent share `git/invalidation.rs`. Content patches do not refresh refs or stashes.
- `Mutex<AppRepoState>` holds per-window repo handles; `RepositoryRuntimeRegistry` is the shared status/watcher/`FileIndex`

### Testing

- Vitest with jsdom environment
- TZ=UTC for all tests
- Test files: `src/**/*.test.{ts,tsx}`
- Exclude `.temp-vscode/` from test discovery

### Discard Operations

- Always show a native confirm dialog before discarding changes (destructive, irreversible)
- Uses `confirm()` from `@tauri-apps/plugin-dialog`

### Themes

- Theme system uses Pierre's shared Shiki catalog through `@pierre/theming`
- Default themes are `vesper` and `ayu-light`
- Shiki workbench colors drive app-wide `--vscode-*` variables
- Pierre Diffs consume Shiki theme IDs directly; Pierre Trees use `themeToTreeStyles()`
- Theme picker opens via Cmd+K Cmd+T chord or status bar icon
- Terminal theme extracted from resolved theme `colors` at runtime via `getTerminalTheme()`

### CLI Tool

- DeathPush installs `dp` and `deathpush` symlinks (or `.cmd` scripts on Windows) to `/usr/local/bin`
- Install/uninstall managed via `commands/cli.rs` with elevated permissions when needed
- CLI opens DeathPush with the given repo path: `dp /path/to/repo`

### Pierre Diffs

- File viewer: Pierre VirtualizedFile + Editor
- SCM and history: Pierre FileDiff + Editor
- Highlighter: `shiki-js` via the Pierre worker pool (`worker.format: "es"`)
- Theme register id equals `theme.id`
- Word wrap is Off/On (`overflow: wrap | scroll`)
- Editor font family, size, line height, and tab size apply as CSS on Pierre hosts
- Diff layout, hunk separators, and inline hunk actions come from settings and apply live

### Pierre Trees

- Explorer and SCM Changes use `@pierre/trees` through `components/trees/file-tree-host.tsx`
- The Explorer backend lists tracked, untracked, and gitignored files through asynchronous `git ls-files`; `.git` and other VCS metadata stay hidden, gitignored entries render gray, and Trees infers directories from those paths
- Quick Open matches a coalesced per-root `FileIndex` on `RepositoryRuntime`, invalidated on structural membership including `.gitignore` effects. Content search stays live `git grep`.
- UI settings expose the Trees density presets (compact, default, relaxed) and icon presets (minimal, standard, complete)
- Default tree density is `compact`; default tree icons are `complete`
- History and Quick Open use neutral Codicon file and folder icons, not Trees icons

### Settings

- App settings stored in localStorage under `deathpush:settings`
- Sections: UI (font, sidebar position, tree density, tree icons), Editor (font, tab size, word wrap), Diff Viewer (layout, inline hunk actions, line numbers, indicators, inline highlighting, backgrounds, hunk separators), Terminal (font, cursor, shell, bell, copy on select, right-click word select), Git (blame toggle), Projects (directory, scan depth)
- Settings page accessible via Cmd+, or DeathPush menu

### Layout Persistence

- Layout state (sidebar width, terminal visibility/height, panel tab, collapsed panes) persisted per-project in localStorage
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
just lint         # Run vp lint + clippy
just fmt          # Format TypeScript and Rust
just check        # Type-check frontend and backend
just test         # Run Vitest and Rust tests
just test-watch   # Run Vitest in watch mode
```
