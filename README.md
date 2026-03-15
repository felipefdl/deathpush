<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="imagem/deathpush_white_nobg.png" />
    <img src="imagem/deathpush_black_nobg.png" alt="DeathPush" width="220" />
  </picture>
</p>

<h1 align="center">DeathPush</h1>

<p align="center"><strong>Murder the Noise. Ship the Code.</strong></p>

<p align="center">Beautiful diffs, clean GUI, zero bloat. No more opening VS Code just to review your own shit.</p>

<p align="center">
  <a href="#get-running-in-60-seconds"><strong>Get Started</strong></a>
</p>

## Why DeathPush

DeathPush is a native macOS Git client for people who like the VS Code Source Control workflow, but hate paying the context-switch tax.

- Review and stage changes fast, without opening your editor.
- Keep commits clean with hunk-level control and clear diffs.
- Handle real Git work (branches, stash, tags, cherry-pick, reset) from one focused UI.
- Stay in flow with native performance and no feature bloat.

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

## Built for VS Code Git Muscle Memory

If you already know VS Code Source Control, DeathPush feels immediately familiar.

- Same mental model.
- Less overhead.
- Faster path from "changed file" to "clean commit."

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
