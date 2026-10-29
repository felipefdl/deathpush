# Terminal

Status: Current product
Date: 2026-10-23

## Purpose

The bottom panel of a repository window: interactive shells and a log of the Git commands the app ran. It lives in the [app shell](app-shell.md) terminal slot. Fonts, cursor, shell, bell, and clipboard access are configured in [settings](settings.md).

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

**Find widget.** A small bar in the top-right corner of a pane, over the output: a `Find` field, the match count, previous and next buttons, and a close button.

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
| Find | none | Open the find widget on the focused pane, or select its query when it is open | Cmd/Ctrl+F when focused |
| Find field | placeholder `Find` | Search scrollback and screen as the text changes | none |
| Next match | tooltip `Next Match (Enter)` | Select the next match toward newer output and scroll to it | Enter in the field |
| Previous match | tooltip `Previous Match (Shift+Enter)` | Select the previous match toward older output and scroll to it | Shift+Enter in the field |
| Close find | tooltip `Close (Escape)` | Close the widget, clear the highlights, focus the pane | Escape in the field |
| Pane row | pane name | Activate that pane | Alt+Cmd/Ctrl+1 to 9 selects group n |
| Menu Toggle Terminal | `Toggle Terminal` | Show or hide | Cmd/Ctrl+J |
| Menu New Terminal | `New Terminal` | Add a group | Cmd/Ctrl+Shift+J |
| Menu Kill Terminal | `Kill Terminal` | Kill the active group | none |

## Copy

- `Output`, `Terminal`
- `New Terminal`, `Split Terminal Horizontally`, `Split Terminal Vertically`, `Maximize Panel Size`, `Restore Panel Size`, `Close Panel`
- Pane sidebar: `Split Horizontally`, `Split Vertically`, `Kill Terminal`
- `No git commands recorded yet.`
- Find widget: placeholder `Find`, count `{i} of {n}`, `No results`, tooltips `Previous Match (Shift+Enter)`, `Next Match (Enter)`, `Close (Escape)`
- Clipboard read dialog: title `Clipboard Access`, message `"{name}" wants to read your clipboard.`, buttons `Allow` and `Deny`
- Default pane name `Terminal {n}`

## Visual

The panel sits above the status bar. The tab strip uses the panel colors with the active tab highlighted. Inactive panes are dimmed. Terminal text uses the terminal settings (monospace font, 13px, line height 1.2, block blinking cursor, color saturation 1.42). The terminal palette derives from the active theme.

The find widget uses the popover colors. Matches are filled with the theme's find match color and the selected match with its active match color, so both come from the active theme.

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

**Find.** The widget opens empty or with the last query of that pane. Typing searches the whole scrollback and the screen, ignoring ASCII letter case, and selects the newest match. The count reads `{i} of {n}` with `i` counted from the top of the scrollback, or `No results` when nothing matches (in the danger color). Enter and Shift+Enter move to the next and previous match in the order they appear on screen, wrapping at the ends, and scroll the viewport to the selected match. Matches visible on screen are highlighted, the selected one distinctly. New output keeps updating the count while the widget is open. Escape or the close button closes the widget and the pane regains focus. An empty field shows no count and no highlights.

**Pointer shape.** A program that sets the mouse pointer shape (OSC 22) changes the pointer over its pane to the closest platform cursor. The pointer starts as the text cursor and returns to it when the program clears the shape. Shapes with no platform equivalent (help, progress, wait, zoom) show the arrow.

**Clipboard read.** A program that asks to read the clipboard (OSC 52) is answered per the `Clipboard Read` setting. `Ask` shows the system dialog (warning style) naming the pane's process; `Allow` answers with the clipboard text; `Deny` answers with an empty clipboard. Closing the dialog with Deny, no answer within 30 seconds, or closing the pane also answers empty. The program waits for the answer, so the pane does not update while the dialog is open. Programs that only ask which clipboard types exist are answered without a dialog. Clipboard writes by programs are unaffected.

**Images.** Programs show images with the Kitty graphics protocol. Supported: PNG (`f=100`), raw RGB (`f=24`), RGBA (`f=32`), and gray, with zlib compression and chunked transmission; placement at the cursor with column and row sizing, pixel offsets, and a cropped source rectangle; relative placements; Unicode placeholder cells (the way tmux passes images through); z-index; deletion by the protocol's delete commands; and animation (frame actions and playback control), which plays by itself with no further input from the program. An image moves with the text it was placed on, including when the viewport scrolls through the scrollback, and disappears when it is deleted or scrolled out of the screen. It is clipped to the pane and takes no mouse input. A z-index below `-1073741824` draws below the cell backgrounds, any other negative z-index between the backgrounds and the text, and `0` and up above the text; a lower z-index draws first within its layer. Find and selection highlights draw over images below the text, and the cursor draws over everything. The main and alternate screens keep separate images, each limited to 320 MB of decoded pixels.

Images travel inside the terminal stream only (`t=d`). The file (`t=f`), temporary-file (`t=t`), and shared-memory (`t=s`) mediums are refused: the terminal never opens a path or a shared-memory object a program names, so a program running over ssh or from an untrusted repository cannot read the local machine. A refused or undecodable image shows nothing and the pane carries on. There is no setting; images are always on.

## Interactions

**Spawn.** Each pane runs the configured shell (empty means the user's default shell) sized to the pane. Resizing the pane resizes the shell.

**Exit.** When the shell exits, the pane closes.

**Copy on select, right-click word select, Option-click selection.** Per the terminal settings.

**Selection while scrolling and typing.** A selection stays on the text it was made on while the viewport scrolls or output arrives, and is dropped as soon as that text leaves the viewport or is no longer under the highlight, so Copy never returns text other than what was highlighted. Typing, pasting or pressing a key for the shell drops the selection and returns the viewport to the bottom; copying, releasing a key, typing in the find field and modifiers alone do not.

**Narrow panes.** The find widget stays inside its pane. Below 380 logical pixels of measured grid width it uses three rows: a full-width query, a reserved-height count, and previous/next/close buttons. Wider panes use a fixed-width horizontal bar. Query text, caret and focus survive layout changes, and count changes do not move the input.

**Git commands.** Every Git command the app runs appends a line to Output.

## Keyboard

- Cmd/Ctrl+J: toggle the panel
- Cmd/Ctrl+3: show and focus
- Alt+Cmd/Ctrl+1 to 9: activate group n
- With focus inside a terminal on the Terminal tab: Cmd/Ctrl+T new, Cmd/Ctrl+D split horizontally, Cmd/Ctrl+Shift+D split vertically, Cmd/Ctrl+W kill pane, Cmd/Ctrl+F find
- With focus in the find field: Enter next match, Shift+Enter previous match, Cmd/Ctrl+F select the query, Escape close
- Cmd/Ctrl+Shift+J: new terminal (menu)

## Persistence

Per project: visible, height, maximized, active tab. Terminal sessions belong to the window and are not restored.
