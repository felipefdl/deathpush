# DeathPush Native macOS -- TODO

Remaining work prioritized by user workflow impact.

## Legend

- `[ ]` Not started
- `[~]` Partial / stubbed
- `[x]` Done

---

## P0 -- Broken / Blocks Trust

- [ ] `open_repository()` returns `groups: vec![]` -- Rust side should call `get_repository_status()` instead of building empty status
- [ ] Loading spinners for long FFI operations (fetch, pull, push, clone)
- [ ] Error toasts instead of modal alerts

---

## P1 -- Core Git Workflow Gaps

- [ ] Line-level staging UI (`stage_lines` wrapper ready, needs diff view selection in Monaco)
- [ ] Stash diff viewer UI (show stash contents in Monaco, `showStash()` wrapper ready)
- [ ] Blame view: gutter annotations showing author/date per line (`blameFile()` wrapper ready)
- [ ] Blame view: click to navigate to commit
- [ ] Blame view: per-file history sidebar (`fileLog()` wrapper ready)
- [ ] Confirm dialog before discarding unsaved changes on window close

---

## P2 -- Terminal as a Real Tool

- [ ] Foreground process detection for tab title (FFI exists: `terminal_foreground_process`)
- [ ] Terminal: Kill Terminal menu item
- [ ] Terminal search
- [ ] Git output panel (capture `onGitCommand` events from EventBridge)

---

## P3 -- Keyboard-Driven Productivity

- [ ] Cmd+Z -- Undo (in commit context: undo last commit)
- [ ] View: Zoom In/Out/Reset (Cmd+/-/0)
- [ ] View: Icon Theme menu item
- [ ] Terminal: Kill Terminal shortcut

---

## P4 -- Ship It

- [~] Sparkle auto-update (stubbed, needs key + uncomment)
- [ ] App icon images in Assets.xcassets (sizes defined, images missing)
- [ ] CI/CD pipeline (GitHub Actions: build, sign, notarize, release)
- [ ] Appcast.xml generation for Sparkle

---

## P5 -- Nice to Have

- [ ] Drag-and-drop file operations in explorer
- [ ] Drag-and-drop to open repository folder
- [ ] Terminal split panes
- [ ] Window state persistence (size, position, sidebar width)
- [ ] File icon theme picker in Settings
- [ ] Help: Open Source Licenses
- [ ] Worktree switcher UI

---

## Done (collapsed)

<details>
<summary>All completed items</summary>

### FFI Wrappers
- [x] `delete_branch` / `rename_branch` / `delete_remote_branch`
- [x] `create_tag` / `delete_tag` / `push_tag` / `delete_remote_tag`
- [x] `stash_save_include_untracked` / `stash_save_staged` / `stash_show`
- [x] `get_commit_detail` / `get_commit_file_diff` / `get_file_blame` / `get_file_log` / `get_last_commit_info`
- [x] `cherry_pick` / `reset_to_commit` / `stage_lines`
- [x] `scan_projects_directory` / `discover_repositories` / `detect_worktrees`
- [x] `get_git_config` / `set_git_config`
- [x] All file explorer FFI commands (list, read, write, delete, rename, copy, move, duplicate, import, gitignore)
- [x] `fuzzy_find_files` / `search_file_contents`

### Views & UI
- [x] Explorer: file tree, git status indicators, context menus, file type icons
- [x] Quick Open: search, recent files, content highlighting, loading indicator
- [x] Diff: Monaco integration, image diff, binary detection, large file handling
- [x] History: commit detail, file diffs, cherry-pick, reset, copy SHA/message
- [x] Tags: list, create, delete (local+remote), push
- [x] Branches: create, delete (force), rename, delete remote, merge, rebase
- [x] Settings: git config, theme picker, projects scanner, auto-fetch interval
- [x] Terminal: SwiftTerm with glass tab bar

### App-Level
- [x] Multi-window with per-tab state
- [x] Menu bar: Open, Clone, Install CLI, New Tab, Quick Open, Color Theme, Sidebar tabs, Fetch/Pull/Push, Stage All, Unstage All, Stash/Pop, Undo Commit, New/Toggle Terminal
- [x] Keyboard shortcuts: Cmd+P, 1/2/3, J, Return, Shift+F/P/U/A/J/C/T, T
- [x] Auto-fetch, Gravatar, relative dates, file type icons
- [x] Deep linking, CLI tool, CLI installer, DMG, notarization

</details>
