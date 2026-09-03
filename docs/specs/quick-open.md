# Quick Open

Status: Current product
Date: 2026-09-02

## Purpose

A command-palette overlay for jumping to a file in the open repository, jumping to a line in the current file, or searching file contents. It sits on top of the [app shell](app-shell.md).

## Layout

Full-window transparent backdrop. The palette is 600 wide, up to 440 tall, horizontally centered, 60 from the top. Inside, top to bottom: search field, a thin loading bar while a search is running, then a scrollable result list.

Clicking the backdrop closes the palette. Clicking inside does not.

## Regions

**Search field.** Full width, placeholder `Search files by name (append : to go to line, # to search content)`.

**Loading bar.** A 2px animated strip under the field while a search is in flight.

**Result list.** File hits, content hits, the go-to-line message, or an empty-state message.

## Controls

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Search field | placeholder above | File search by default; content search when the query starts with `#`; go-to-line when the query is `:digits` | focused on open |
| File row | file name (matched characters highlighted), optional `:{line}`, directory in muted text | Open the file in the [Explorer](explorer.md) file viewer. With a `:N` suffix, jump to line N | Enter on the active row, or click |
| Content row | `{filename}:{line}`, directory, the trimmed matching line | Open the file at that line | Enter on the active row, or click |
| Backdrop | none | Close | Escape |
| View menu Quick Open... | `Quick Open...` | Open this palette (only when a repository is open) | Cmd/Ctrl+P |

## Copy

- Placeholder: `Search files by name (append : to go to line, # to search content)`
- No file matches: `No matching files`
- Content mode, query is only `#`: `Type to search file contents`
- Content mode, searching: `Searching...`
- Content mode, no hits: `No results`
- Go to line with a valid N: `Go to line {N} in current file. Press Enter to confirm.` (N in bold)
- Go to line with N of 0: `Type a line number to go to.`
- Section label above recent files: `recently opened`
- Section label above the remaining files: `files`
- Menu item: `Quick Open...`

## Visual

Panel on the sidebar background with a subtle border, rounded corners, and a soft shadow. Field 32 tall. Rows 26 tall, 13px, with a neutral file icon, the name, and the path in muted text. The active row uses the list selection colors. Matched characters use the list highlight color. Section labels are 11px muted. Content snippets are 12px muted. The loading bar uses the progress color and slides left to right on a 1.5 s loop.

## States

**Closed.** Not shown. Opens only when a repository is open.

**Open, empty query.** Lists the first 100 files alphabetically. Files opened recently in this repository come first under `recently opened`; the rest follow under `files`. The labels appear only when at least one recent file exists in the index.

**File query.** After a 100 ms pause in typing, fuzzy-match the query against every path, rank by score, cap at 100. Matched characters in the file name are highlighted. No recent grouping.

**`name:N`.** The same file search on the text before the last `:N`. Every row shows `:{N}`. Selecting opens that file at line N.

**`:N` only.** No list. Shows the go-to-line message. Enter jumps to line N in the file already open in the viewer, docks the terminal, and switches the main panel to the file viewer. If no file is open, the palette just closes.

**`#query`.** Content mode. After a 300 ms pause, search file contents for the literal text (not a pattern). `#` alone clears the results and shows `Type to search file contents`.

**Loading.** The loading bar shows. In file mode the empty message stays `No matching files`; in content mode it becomes `Searching...`.

**Error.** A failed search clears the list. No error message in the palette.

**Keyboard vs pointer.** Arrow keys take ownership of the highlight; hover regains it on the next mouse move over the list.

## Interactions

**Select.** Opens the file in the file viewer (at a line when given), records it as a recent file, docks the terminal, switches the sidebar to Explorer and the main panel to the file viewer, then closes.

**Search scope.** File search covers tracked files plus untracked files that are not ignored. The index rebuilds when files are added, removed, or renamed, or when ignore rules change; editing a file's content does not rebuild it. Content search is live over the same set, case-sensitive and literal, skips nested repositories, and returns path, line number, and the line text. A query with no hits is an empty list, not an error.

**Open.** View > Quick Open... or Cmd/Ctrl+P, even while typing in a text field. Ignored when no repository is open.

## Keyboard

| Key | Action |
|---|---|
| Cmd/Ctrl+P | Open (Cmd/Ctrl+Shift+P is the diff layout toggle, not this) |
| Escape | Close |
| Down / Up | Move the active row, wrapping. Disabled in go-to-line mode |
| Enter | Open the active row, or jump to the line in go-to-line mode |
| Typing | Update the query. The field has autocomplete, autocorrect, autocapitalize, and spellcheck off |

## Persistence

Recent files: up to 20 paths per repository with last-opened times, newest first. Also updated when Explorer opens a file. Stored locally on the machine.
