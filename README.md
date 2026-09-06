<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="imagem/deathpush_white_nobg.png" />
    <img src="imagem/deathpush_black_nobg.png" alt="DeathPush" width="220" />
  </picture>
</p>

<h1 align="center">DeathPush</h1>

<p align="center"><strong>Murder the Noise. Push the Code.</strong></p>

<p align="center">Beautiful diffs, clean GUI, zero bloat. No more opening VS Code just to review your own shit.</p>

<p align="center">
  <a href="#get-running-in-60-seconds"><strong>Get Started</strong></a>
</p>

## Why DeathPush

DeathPush is a standalone desktop Git client for people who like the VS Code Source Control workflow, but hate paying the context-switch tax.

- Review and stage changes fast, without opening your editor.
- Keep commits clean with clear diffs.
- Handle real Git work (branches, stash, tags, cherry-pick, reset) from one focused UI.
- Stay in flow with native performance and no feature bloat.

## What You Can Do

- Track changes across staged, unstaged, and untracked files.
- Diff files inline or side by side.
- Diff images side-by-side (PNG, JPG, GIF, WebP, AVIF, SVG, and more).
- Stage and unstage files. Discard, commit, and amend without losing momentum.
- Push, pull, fetch, checkout, and create branches quickly.
- Manage stashes and tags in the same workflow.
- Browse history and inspect commit details when you need context.
- See blame for the current line in the status bar, then open file history when you need more context.
- Handle merge and rebase conflicts (continue, abort, skip) from the app.
- Manage files directly: delete, add to `.gitignore`, open in editor, or reveal in file manager.
- Open multiple windows for different repositories.

## Built for VS Code Git Muscle Memory

If you already know VS Code Source Control, DeathPush feels immediately familiar.

- Same mental model.
- Less overhead.
- Faster path from "changed file" to "clean commit."

## Downloads

Installers are on [GitHub Releases](https://github.com/felipefdl/deathpush/releases):

- macOS: `.dmg` (Apple Silicon and Intel)
- Linux x86_64: `.deb` and AppImage
- Windows x86_64: NSIS installer (`-setup.exe`). ARM is included when that job succeeds.

A packaged build checks for updates 2 seconds after the welcome screen appears. When one exists, the footer shows `Update to v{version}`; click it to download and install. Debug builds skip the check.

## Prerequisites

- Rust stable (1.97 or newer)
- [Zig](https://ziglang.org/) 0.16.0 on PATH, for libghostty-vt
- [`just`](https://github.com/casey/just) task runner (`cargo install just`)
- Linux: X11, Wayland, fontconfig, Vulkan, OpenSSL, and zstd development packages. On Ubuntu:

  ```sh
  sudo apt-get install -y libfontconfig-dev libwayland-dev libx11-xcb-dev \
    libxkbcommon-x11-dev libvulkan1 libssl-dev libzstd-dev
  ```

## Get Running in 60 Seconds

```sh
just dev
```

Build production binary:

```sh
just build
just package  # cargo packager --release (pass --formats to pick dmg, deb, appimage, nsis)
```

Quality checks:

```sh
just lint    # clippy, warnings denied
just test    # cargo test --workspace
just fmt     # rustfmt
just check   # cargo check, all targets
```

Cut a tagged release with `just release <version>` (bumps the workspace version, commits, tags; does not push). Push the tag, then review the draft GitHub release.

## Under the Hood

DeathPush is built with a hybrid Git engine:

- `git2` for fast read operations (status, diff, branches, log, tags).
- Native `git` CLI for write operations (commit, push/pull, stash, checkout, reset, clone), so hooks, signing, credentials, SSH config, and LFS keep working as expected.
- Diffs come from git2 with tree-sitter highlighting.
- Auto-update: the welcome footer installs a newer package after a minisign-verified check of `latest.json` on GitHub Releases.

Stack: GPUI (Rust) through gpui-kit, git2 and the git CLI, libghostty-vt for the terminal.

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
