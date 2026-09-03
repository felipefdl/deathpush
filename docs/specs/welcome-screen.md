# Welcome screen

Status: Current product
Date: 2026-09-02

## Purpose

The project picker shown when no repository is open. Open a folder, clone a remote, pick a recent project, or pick a project found in a configured workspace directory. After a successful open the [app shell](app-shell.md) takes over.

The brand tagline (`Murder the Noise. Push the Code.`) belongs to the README and the website. This screen shows only the app mark and the wordmark `DeathPush`.

## Layout

Full-window column: a draggable strip at the top (macOS) or a 16px spacer (Linux), then the body, then the footer. No sidebar and no status bar.

Body, centered: mark, title, two action buttons side by side, then a two-column row of lists (`Recent` on the left, `Workspace` on the right).

```mermaid
flowchart TB
  mark[App mark, 80px]
  title[DeathPush]
  actions[Open Repository | Clone Repository]
  subgraph lists [Lists row]
    direction LR
    recent[Recent: header, filter, list]
    workspace[Workspace: header, filter, tree or list, Configure Workspace...]
  end
  footer[Footer: update button, version]
  mark --> title --> actions --> lists --> footer
```

## Regions

**Mark.** The app mark, 80px, white on dark themes and black on light themes. Accessible name `DeathPush`.

**Title.** `DeathPush`.

**Actions.** Two equal primary buttons with icons: a folder icon with `Open Repository`, a cloud-download icon with `Clone Repository`.

**Recent.** Header `Recent` (rendered uppercase), a filter field, and a scrollable list of the last 20 opened projects, newest first. Each row: repository icon, project name, the path in muted text, and a close button on hover (tooltip `Remove from recents`).

**Workspace.** Header `Workspace` (rendered uppercase), a filter field, a scrollable tree or flat list of repositories found under the configured workspace directories, and a footer link `Configure Workspace...`.

**Footer.** An optional update button, then the version string.

**Opening overlay.** A full-window dimmer with a spinner and `Opening repository...` while a repository is being opened.

## Controls

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Open Repository | `Open Repository` | System folder picker, then open that folder | Cmd/Ctrl+O (File menu) |
| Clone Repository | `Clone Repository` | Open the [clone dialog](clone-dialog.md) | none |
| Recent filter | placeholder `Filter recent (⌘1)` on macOS, `Filter recent (Ctrl+1)` elsewhere | Filter recents by name or path | Cmd/Ctrl+1 focuses it |
| Recent row | name and path | Open that project | Enter or Space on the focused row |
| Remove recent | close icon, tooltip `Remove from recents` | Remove from the list. No confirmation | none |
| Workspace filter | placeholder `Filter workspace (⌘2)` on macOS, `Filter workspace (Ctrl+2)` elsewhere | Filter found projects by name or path. A non-empty filter flattens the tree into a list | Cmd/Ctrl+2 focuses it |
| Workspace folder row | folder name | Expand or collapse | Right arrow or Enter expands, Left arrow collapses, Space toggles |
| Workspace project row | repository name | Open that project | Enter |
| Configure Workspace... | `Configure Workspace...` | Open Workspace Settings. See [overlays](overlays.md) | none |
| Update | `Update to v{version}`, then `Updating {n}%` | Download and install the update | none |

## Copy

- `DeathPush`
- `Open Repository`
- `Clone Repository`
- `Recent`
- `Filter recent (⌘1)` / `Filter recent (Ctrl+1)`
- `No recent projects`
- `No matching projects`
- `Remove from recents`
- `Workspace`
- `Filter workspace (⌘2)` / `Filter workspace (Ctrl+2)`
- `No workspace directories configured`
- `No git repositories found`
- `Configure Workspace...`
- `Opening repository...`
- `Version {version} ({git hash})`
- `Update to v{version}`
- `Updating {n}%`

## Visual

Action buttons: primary button colors, 13px text, icon then label, 8px apart. Mark 80px with 16px below it. Title 20px, semibold, 2px letter spacing, 20px below it.

Lists: two equal columns 12px apart, each 200 to 360 tall, on the sidebar background with a subtle border and rounded corners. Headers are 11px bold uppercase. Filter fields are 26 tall with a search icon. Recent rows show the repository icon, the name, and the path in muted text; the remove button appears on hover. Tree rows indent 16px per level from a 12px base; folder chevrons rotate when collapsed.

Footer text is 11px muted. The opening overlay dims the screen by 40% behind the spinner.

## States

**No recents.** `No recent projects`.

**Filtered recents, no match.** `No matching projects`.

**No workspaces configured.** `No workspace directories configured`. No scan runs.

**Workspaces configured, none found.** `No git repositories found`. There is no scanning spinner; the list keeps its previous content until the scan finishes.

**Tree or flat.** The workspace list is a tree when there is more than one workspace directory or any directory has a scan depth above 1, the filter is empty, and no row is keyboard-highlighted. Otherwise it is a flat list of name and path.

**Opening.** The overlay blocks the screen until the repository opens or fails.

**Update available.** The update check runs 2 s after the screen appears. When an update exists the footer shows the update button. It is disabled while a download is in progress.

**Light theme.** Black mark.

## Interactions

**Open Repository.** Folder picker, then open. On success the project is added to recents (with its branch) and the app shell appears.

**Clone Repository.** Opens the clone dialog. On success the clone opens in this window.

**Select project.** Clicking a recent or workspace row opens that path, the same as Open Repository with a known path.

**Remove recent.** Removes the row at once. No confirmation.

**Configure Workspace...** Opens Workspace Settings. Saving rewrites the workspace list and reruns the scan.

**Scan.** Each workspace directory is searched to its configured depth for Git repositories. A failed scan yields an empty list. Rows show name and path only; the branch is not shown here.

**Filters.** Live filtering as the user types. Up and Down move a highlight through the rows (folder and project rows alike in the tree). Enter opens the highlighted row. Escape blurs the field.

## Keyboard

- Cmd/Ctrl+1: focus the recent filter
- Cmd/Ctrl+2: focus the workspace filter
- Up, Down, Enter, Escape: list navigation from a filter field
- Enter or Space: activate the focused row

Cmd/Ctrl+1 and Cmd/Ctrl+2 mean Changes and Explorer inside a repository. They never clash, because this screen exists only when no repository is open.

## Persistence

- Recents: up to 20 entries of path, name, last-opened time, and branch, sorted newest first. Stored locally on the machine.
- Workspace directories: part of app settings (`Projects`). See [settings](settings.md).
