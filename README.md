<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="imagem/deathpush_white_nobg.png" />
    <img src="imagem/deathpush_black_nobg.png" alt="DeathPush" width="220" />
  </picture>
</p>

<h1 align="center">DeathPush</h1>

<p align="center"><strong>Murder the Noise. Ship the Code.</strong></p>

<p align="center">A Git GUI for terminal power users. Your AI agent writes the code, you keep eyes on every change.</p>

<p align="center">
  <a href="#get-running-in-60-seconds"><strong>Get Started</strong></a>
</p>

## Why DeathPush

You work in the terminal. Claude Code, Codex, Vim, Copilot CLI, whatever. Your agent writes hundreds of lines while you wait. You need to see what it's doing without opening VS Code or any other IDE.

DeathPush sits next to your terminal. It picks up file changes in real time, shows you diffs, lets you stage what's right, fix what's not, and commit when you're ready.

- Watch changes land as your agent writes code.
- Stage files and hunks visually, no `git add -p` gymnastics.
- Fix a typo inline without switching to an editor.
- Handle branches, stash, tags, cherry-pick, reset from one focused UI.
- Native performance, no Electron, no bloat.

## What You Can Do

- Track changes across staged, unstaged, and untracked files.
- Diff files inline or side-by-side with Monaco-powered views.
- Diff images side-by-side (PNG, JPG, GIF, WebP, AVIF, SVG, and more).
- Stage, unstage, discard, and commit, including amend, without losing momentum.
- Push, pull, fetch, checkout, and create branches quickly.
- Manage stashes and tags in the same workflow.
- Browse history and inspect commit details when you need context.
- View file blame to trace who changed what.
- Handle merge and rebase conflicts (continue, abort, skip) without touching the terminal.
- Search inside your terminal output with the built-in search bar.
- Use the integrated terminal with full PTY support when you do need it.
- Install the `dp` command line tool to open repos from your terminal.
- Manage files directly: delete, add to `.gitignore`, open in editor, or reveal in Finder.
- Open multiple tabs for different repositories.

## Works With Any Terminal Workflow

Claude Code, Codex, Vim/Neovim, Copilot CLI, OpenCode, Aider, or any CLI agent. If it writes files, DeathPush shows you what changed.

## Prerequisites

- [Xcode](https://developer.apple.com/xcode/) (latest stable)
- [Rust toolchain](https://rustup.rs/) (edition 2024, minimum rustc 1.85.0)
- [`just`](https://github.com/casey/just) task runner (`cargo install just`)

## Get Running in 60 Seconds

```sh
open DeathPush/DeathPush.xcodeproj
```

Build and run from Xcode (Cmd+R). The Rust FFI crate is compiled automatically via Xcode build phases.

Quality checks:

```sh
just lint    # cargo clippy
just test    # cargo test
just fmt     # cargo fmt
just check   # cargo check
```

## Under the Hood

DeathPush is built with a hybrid Git engine:

- `git2` for fast read operations (status, diff, branches, log, tags).
- Native `git` CLI for write operations (commit, push/pull, stash, checkout, reset, clone), so hooks, signing, credentials, SSH config, and LFS keep working as expected.
- Custom syntax highlighting for TOML, Justfile, and dotenv files in diffs.
- Auto-update via Sparkle with EdDSA-signed releases.

Stack: Swift/SwiftUI + Rust (UniFFI) + Monaco Editor (WKWebView) + SwiftTerm + Sparkle.

## License

Apache-2.0 -- see [LICENSE](LICENSE) for details.
