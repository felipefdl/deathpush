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
- Diff files inline or side by side with Pierre views.
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

## Prerequisites

- [Vite+](https://viteplus.dev/) (`vp`) with Node.js 24 and pnpm
- [Rust toolchain](https://rustup.rs/) (edition 2024, minimum rustc 1.85.0)
- [`just`](https://github.com/casey/just) task runner (`cargo install just`)

## Get Running in 60 Seconds

```sh
vp install
just dev
```

Build production binary:

```sh
just build
```

Quality checks:

```sh
just lint    # vp lint + cargo clippy
just test    # vp test
just fmt     # vp fmt + cargo fmt
just check   # vp check + cargo check
```

## Under the Hood

DeathPush is built with a hybrid Git engine:

- `git2` for fast read operations (status, diff, branches, log, tags).
- Native `git` CLI for write operations (commit, push/pull, stash, checkout, reset, clone), so hooks, signing, credentials, SSH config, and LFS keep working as expected.
- Pierre diffs with Shiki highlighting.
- Auto-update support: get notified and install new versions without leaving the app.

Stack: Tauri v2 (Rust) + Solid 2 + TypeScript + Zustand + Pierre diffs and trees.

## License

Apache-2.0. See [LICENSE](LICENSE) for details.
