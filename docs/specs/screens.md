# Screens

Status: Current product
Date: 2026-09-02

Index of UI specs. Each file is the contract for one surface: what it shows, what it says, and how it behaves. The specs are stack-neutral. They describe the feature, not an implementation, so the app can be rebuilt on any language and any UI toolkit (web, native, or terminal).

Conventions:

- `Cmd/Ctrl` means Cmd on macOS and Ctrl elsewhere.
- Text in backticks is verbatim user-facing copy.
- Colors are named by role (primary button, warning, muted text, selection). The active color theme supplies the values.
- Sizes are reference values for a desktop window. Other form factors keep the proportions and the order.
- Icons are named by meaning (folder, search, close, check). Any icon set that reads the same way is fine.

| Spec | Surface |
|---|---|
| [Welcome](welcome-screen.md) | Project picker (no repository) |
| [Clone dialog](clone-dialog.md) | Clone overlay |
| [App shell](app-shell.md) | Title, sidebar tabs, status bar, layout |
| [SCM Changes](scm-changes.md) | Commit, groups, stash, merge, overflow, diff |
| [Explorer](explorer.md) | File tree and file viewer |
| [History](history.md) | Commit list and commit diff |
| [Terminal](terminal.md) | Shell panel and Git output |
| [Theme picker](theme-picker.md) | Color theme overlay |
| [Settings](settings.md) | Settings page |
| [Native menus](native-menus.md) | macOS, Windows, and Linux menus |
| [Branch picker](branch-picker.md) | Branch and tag overlay |
| [Quick Open](quick-open.md) | File, line, and content search |
| [Overlays](overlays.md) | Workspace Settings, licenses, boot splash |
