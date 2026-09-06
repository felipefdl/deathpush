# Terminal

Status: Current product
Date: 2026-09-02

## Purpose

The bottom panel of a repository window: interactive shells and a log of the Git commands the app ran. It lives in the [app shell](app-shell.md) terminal slot. Fonts, cursor, shell, and bell are configured in [settings](settings.md).

## Layout

Column: a header with tabs on the left and actions on the right, then the body.

Header tabs, left to right: `Output`, `Terminal`.

Header actions when the Terminal tab is active (or the panel is maximized): new terminal, a separator, split horizontally, split vertically, maximize or restore, close panel.

Body: the active terminal group (one or more panes) or the Git output log. A pane sidebar listing the panes appears only when the active group has more than one pane; it is 160 wide by default and draggable.

Terminal groups: each new terminal is a group. A group can be split into panes, horizontally or vertically. Panes in a group share the group's space.

Pane names follow the process on Unix: a new pane is `Terminal {n}`, then takes the shell name once spawned (for example `zsh` or `bash`), then updates every second to the name of the foreground process (for example `cargo` or `node`). On Windows the pane name stays the shell name. There is no foreground-process discovery.

## Regions

**Tabs.** `Output` and `Terminal`.

**Header actions.** New, Split Horizontally, Split Vertically, Maximize or Restore, Close Panel.

**Terminal body.** A full terminal emulator per pane, running the configured shell in the repository root.

**Git output.** One line per Git command the app ran: timestamp, `[info]`, `>`, the command, then `[{duration} ms]`. Empty state: `No git commands recorded yet.`

**Pane sidebar.** One row per pane with its name. Hover reveals split and kill actions.

## Controls

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Output tab | `Output` | Show the Git log | none |
| Terminal tab | `Terminal` | Show the terminals and focus the active pane | Cmd/Ctrl+3 |
| New Terminal | tooltip `New Terminal` | Add a group | Cmd/Ctrl+T when a terminal is focused; Cmd/Ctrl+Shift+J from the menu |
| Split Horizontally | tooltip `Split Terminal Horizontally` | Split the active pane | Cmd/Ctrl+D when focused |
| Split Vertically | tooltip `Split Terminal Vertically` | Split the active pane | Cmd/Ctrl+Shift+D when focused |
| Maximize | tooltip `Maximize Panel Size` / `Restore Panel Size` | Toggle maximized | none |
| Close Panel | tooltip `Close Panel` | Hide the panel | Cmd/Ctrl+J |
| Kill pane | tooltip `Kill Terminal` (pane sidebar) | Kill that pane; kill the group when it was the last pane | Cmd/Ctrl+W when focused |
| Pane row | pane name | Activate that pane | Alt+Cmd/Ctrl+1 to 9 selects group n |
| Menu Toggle Terminal | `Toggle Terminal` | Show or hide | Cmd/Ctrl+J |
| Menu New Terminal | `New Terminal` | Add a group | Cmd/Ctrl+Shift+J |
| Menu Kill Terminal | `Kill Terminal` | Kill the active group | none |

## Copy

- `Output`, `Terminal`
- `New Terminal`, `Split Terminal Horizontally`, `Split Terminal Vertically`, `Maximize Panel Size`, `Restore Panel Size`, `Close Panel`
- Pane sidebar: `Split Horizontally`, `Split Vertically`, `Kill Terminal`
- `No git commands recorded yet.`
- Default pane name `Terminal {n}`

## Visual

The panel sits above the status bar. The tab strip uses the panel colors with the active tab highlighted. Inactive panes are dimmed. Terminal text uses the terminal settings (monospace font, 13px, line height 1.2, block blinking cursor, color saturation 1.42). The terminal palette derives from the active theme.

## States

**Hidden.** Cmd/Ctrl+J or Cmd/Ctrl+3 shows it. The first show creates a group when none exists.

**One pane.** No pane sidebar.

**Several panes.** Pane sidebar with names and hover actions.

**Maximized.** The terminal fills the main area. Opening History or Settings docks it.

**Output tab.** The Git log shows; terminals stay alive but inactive.

**Empty log.** `No git commands recorded yet.`

**Always Open Terminal on Start.** The panel is forced visible when a project loads.

**Closing the window.** If any pane has a foreground process, the app asks before closing. On Windows the app does not prompt. There is no foreground-process discovery.

**Bell.** `Sound` and `Both` flash the pane like `Visual`. There is no platform sound. `Off` does nothing.

## Interactions

**Spawn.** Each pane runs the configured shell (empty means the user's default shell) sized to the pane. Resizing the pane resizes the shell.

**Exit.** When the shell exits, the pane closes.

**Copy on select, right-click word select, Option-click selection.** Per the terminal settings.

**Git commands.** Every Git command the app runs appends a line to Output.

## Keyboard

- Cmd/Ctrl+J: toggle the panel
- Cmd/Ctrl+3: show and focus
- Alt+Cmd/Ctrl+1 to 9: activate group n
- With focus inside a terminal on the Terminal tab: Cmd/Ctrl+T new, Cmd/Ctrl+D split horizontally, Cmd/Ctrl+Shift+D split vertically, Cmd/Ctrl+W kill pane
- Cmd/Ctrl+Shift+J: new terminal (menu)

## Persistence

Per project: visible, height, maximized, active tab. Terminal sessions belong to the window and are not restored.
