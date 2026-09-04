# DeathPush - Agent Instructions

## Project Overview

DeathPush is a standalone desktop Git client built on GPUI (Rust) that replicates the VS Code Source Control UX. The surface specs under `docs/specs/` are the product contract; `docs/specs/app-architecture.md` is the design; `docs/adr/` holds decisions.

## Brand & Voice

- DeathPush (or "dp" for short) is a dark humor brand name, intentionally edgy -- similar to Liquid Death. It is not offensive, it is branding.
- Dark humor in copy, UI text, and documentation is intentional and must be preserved as-is.
- Agents must NOT sanitize, tone-police, soften, or rewrite text for "marketing purposes" or politeness.
- This is a git client, not a sensitivity project. Respect the developer audience.

## Tech Stack

| Component | Technology |
|-----------|-----------|
| UI | GPUI through gpui-kit (`gpui-component`, `gpui-base`) |
| Core | Rust: git2 reads, git CLI writes, notify watcher, portable-pty, libghostty-vt |
| Async | tokio runtime owned by `Core`; the app awaits join handles on gpui's executor |
| Build | Cargo workspace, Zig 0.16 for libghostty-vt |
| Formatter | rustfmt (`max_width = 120`, `tab_spaces = 2`) |
| Task runner | justfile |
| Website | `website/` (Astro, own pnpm lock) |

## Layout

- `crates/core/` -- `deathpush-core`. No UI dependency. `Core` (runtime, registries, event hub), `ops/` (every operation as a `Core` method), `session/` (intent apply, registry, types, `SessionId`), `git/` (git2 reads, CLI writes, status coordinator, watcher, repository runtime, invalidation), `diff_view/` (pure rows for the diff: inline and side-by-side alignment, word ranges, separator labels), `terminal/` (VT screen), `pty.rs`, `events.rs` (`CoreEvent`, `EventHub`), `shell_env.rs`, `types.rs`, `error.rs`, `relative_time.rs`, `config/` (settings, recents, window bounds, `layout.rs` per-project layout), `theme/` (tm-themes parsing, `UiPalette`), `theme/syntax.rs` (tokenColors scopes mapped to tree-sitter captures), `workspace.rs`, `deep_link.rs`.
- `crates/app/` -- `deathpush`, the gpui binary. `assets.rs` (embedded assets, gpui-kit fallback), `config.rs` (`AppConfig` global, debounced save), `theme.rs` (catalog, `apply_theme`, `ActivePalette`), `zoom.rs`, `actions.rs`, `keymap.rs` (binding table per OS, key contexts), `menus.rs` (native menus, Linux rows), `window.rs` (window options, registry), `shell.rs` (`Shell` root: screens, overlays, toast), `title_bar.rs`, `welcome/`, `overlays/`, `repo/` (`state.rs` (pure session guards), `model.rs` (`RepoModel`, intents, `NeedsConfirmation` prompt), `layout_model.rs`, `output_log.rs`, `view.rs` (chrome), `sidebar.rs`, `changes/` (the SCM sidebar body: toolbar, banner, commit box, filter, groups, rows, overflow menu, branch list), `main_panel.rs`, `diff/` (the diff panel: header, states, rows, highlighter cache, selection), `terminal_panel.rs`, `status_bar.rs`), `cli_install.rs`, `open_requests.rs`.
- `assets/` -- `bin/` (dp launchers), `app-icons/`, `brand/`, `fonts/`, `metainfo/`.
- `docs/specs/` -- surface specs and the architecture spec. `docs/adr/` -- decision records.

## Architecture

### Core boundary

- `Core::new(resource_dir)` starts the shell env resolver, builds the tokio runtime, installs the git command sink.
- Sessions: `open_session()` returns a `SessionId` and a `CoreEvent` receiver. `close_session(id)` is async and awaited; it drops runtime binding, session state, PTYs, and the channel.
- Operations are methods on `Core` keyed by `SessionId`. Async only where the body awaits.
- `CoreEvent`: `SessionStatus` (status patches, including refs and stash refreshes in `extras`), `PathsChanged`, `WatcherError`, `GitCommand`, `TerminalData`, `TerminalExited`.

### Git Strategy: Hybrid

- Read ops (status, SCM diffs, branches, log, tags, ahead/behind): git2. `scm_file_diff` builds original/modified/hunks from one git2 diff.
- Write ops (add, commit, push, pull, checkout, fetch, stash, cherry-pick, reset, clone, `git apply` patches): git CLI via `tokio::process::Command` for hooks, GPG, credentials, SSH, and LFS.
- Blame/file-log: git CLI (porcelain blame, follow log).
- `session_intent` outcomes: `ack`, `patch`, `diff`, `blame`, `needsConfirmation`, `snapshot`. Open, clone, refresh, and HEAD-moving writes return one Snapshot.
- Watcher and intent share `git/invalidation.rs`. Sessions on the same canonical root share one `RepositoryRuntime`.

## Conventions

### Rust

- Edition 2024, minimum 1.97, stable toolchain
- Run clippy with `-D warnings` on the workspace
- Use `thiserror` for error types, `tracing` for logging
- `#[serde(rename_all = "camelCase")]` stays on every type that reaches disk or a manifest
- `crates/core` never depends on gpui or gpui-kit

### UI

- Overlays are the shell's own layer, not gpui-component dialogs. Colors come from `cx.theme()` or `ActivePalette`; no literal colors outside the boot splash.
- Views read `RepoModel::state()`; only `RepoModel` mutates it, through `dispatch` or `apply_status_event`.

### Testing

- `cargo test --workspace`, `TZ=UTC` for the perf tests (`just perf-boot`, `just perf-storm`)
- gpui tests use `#[gpui_kit::test]` with the `test-support` feature

### Docs

- Specs in `docs/specs/`, ADRs in `docs/adr/`. Plans never enter the repo.

## Development

```sh
just dev [path]   # cargo run -p deathpush -- [path]
just build        # release build
just lint         # clippy, warnings denied
just fmt          # rustfmt
just check        # cargo check, all targets
just test         # cargo test --workspace
```

Zig 0.16.0 must be on PATH for the first build (libghostty-vt).
