# 1. GPUI over Tauri

Status: Accepted
Date: 2026-09-03
Author: Felipe Lima

## Context

DeathPush shipped through 0.4.0 as a Tauri app: Rust for git, sessions, the file watcher, and PTYs, and a Solid frontend for every screen. The frontend rented its two hardest surfaces: the diff viewer from Pierre and the terminal renderer from WTerm on top of the Ghostty core. The surface specs under `docs/specs/` describe the product without naming a toolkit, so the product can be rebuilt on another stack.

Forces:

- The web stack was the only reason for pnpm, Vite, Solid, and 13,700 lines of TypeScript. The Rust side is 14,000 lines, and only the command wrappers and the app builder touch Tauri.
- The diff viewer and the terminal were black boxes. Bugs in hunk staging and terminal focus were fixed by working around library behavior instead of at the cause.
- Linux shipped an 87 MB AppImage because WebKitGTK travels with the app. Shipped GPUI apps weigh 23 to 31 MB per platform.
- The owner wants one language, one toolchain, and ownership of the rendering.

State of GPUI on 2026-09-03:

- Zed has published no `gpui` crate since 0.2.2 in October 2025 and rejects pull requests that do not serve Zed. The framework itself is active: 100 commits to `crates/gpui` since June 2026, AccessKit integration, and a split into `gpui`, `gpui_platform`, `gpui_macos`, `gpui_linux`, `gpui_windows`, `gpui_web`, and `gpui_wgpu`.
- Snapshot channels: `gpui-pre` (verbatim snapshots of Zed's tree published by the gpui-kit maintainer, 0.3.3), `gpui-ce` (community fork with daily Zed syncs whose README says its API is diverging), `gpui-unofficial` (auto-published per Zed release), and a git revision of the Zed repository.
- gpui-kit 0.6.0 (Apache-2.0) ships `gpui-component` with 60+ styled widgets and `gpui-base` with the unstyled behavior, including a code editor, tree, virtual list, dock, resizable panes, command palette, and native menu bar. Longbridge Pro runs on it in production. Its editor has no gutter or block-row API.
- No standalone diff viewer and no production terminal element exist. GitComet (AGPL-3.0) and hunk (GPL-3.0) prove both can be built on GPUI and cannot be copied.
- Terminal cores: `alacritty_terminal` 0.26, used by Zed, GitComet, and tty7, or `libghostty-vt` 0.2.1, safe bindings by Ghostty maintainers that need Zig 0.16 at build time.
- Packaging: cargo-packager 0.11.8 with cargo-packager-updater 0.2.3, which verifies minisign signatures like Tauri's updater. Velopack and self_update are the alternatives. cargo-packager builds dmg, deb, AppImage, nsis, and wix, and no rpm.

Options considered:

- Stay on Tauri. Rejected: it keeps the JS toolchain and the rented diff and terminal.
- GPUI through `gpui-ce`. Rejected: the fork announces API divergence, and the gpui-kit maintainer refuses to support it, so the component library and the framework would drift apart.
- GPUI through a git revision of Zed. Rejected: every bump pairs the framework and gpui-kit by hand.
- A GPUI shell with Pierre inside `gpui-wry`. Rejected: it keeps the JS toolchain for the hardest surface.
- `alacritty_terminal` instead of `libghostty-vt`. Rejected: the app already ships Ghostty behavior, and the Zig build step is accepted.
- Extract the core first and keep both UIs in the tree until parity. Rejected: main carries one UI at a time. Hotfixes for the shipped app come from a release branch instead.

## Decision

We build DeathPush on GPUI through the `gpui-pre` snapshot crates pinned by gpui-kit. We delete the Solid frontend and Tauri in one commit and ship the next release when the GPUI app meets every surface spec. Git, sessions, the watcher, PTYs, terminal state, and diff rows live in a crate with no UI dependency. The diff viewer and the terminal grid are our own GPUI elements. The terminal core is `libghostty-vt`. Packaging and update use cargo-packager and cargo-packager-updater with the existing minisign key. Hotfixes for the shipped Tauri app go on `release/0.4`.

The design is in [app architecture](../specs/app-architecture.md).

## Consequences

Easier: one language and one toolchain, no IPC and no camelCase DTO layer between the UI and the core, a diff viewer and a terminal we can fix at the cause, a Linux package a third of its former size, and view tests that run headless in `cargo test`.

Harder:

- Main is unreleasable until the GPUI app passes the specs. Hotfixes for the shipped app come from `release/0.4`.
- The diff viewer and the terminal renderer are ours to build and maintain.
- GPUI has no release channel. Upgrades follow the `gpui-pre` snapshots, and a breaking change in Zed's tree lands on us when gpui-kit adopts it.
- The working-tree diff is read-only. Editing lives in the Explorer file viewer. The merge view offers per-conflict accept choices instead of free-form editing.
- rpm packages stop. deb and AppImage remain.
- Users lose settings and recent projects once, since those lived in WebKit localStorage.
- Zig 0.16 joins the build on every platform and in CI.
- Linux requires a Vulkan-capable driver, where WebKitGTK ran on anything.
