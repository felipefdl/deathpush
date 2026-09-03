# Overlays

Status: Current product
Date: 2026-09-02

Workspace Settings, Open Source Licenses, and the boot splash. Clone, theme picker, Quick Open, and the branch picker have their own specs.

## Purpose

Workspace Settings edits the directories the [welcome screen](welcome-screen.md) scans for repositories. Open Source Licenses lists the bundled third-party licenses. The boot splash fills the window before the app knows whether to show the welcome screen or a repository.

## Layout

**Workspace Settings.** Full-window transparent backdrop; a 440-wide dialog, centered, 60 from the top. Inside: title, description, a scrollable list of directory rows (up to 200 tall), `Add Directory`, then `Cancel` and `OK`.

**Open Source Licenses.** The same backdrop; a 560-wide dialog up to 70% of the window height. Inside: title, a scrollable grouped list, `Close`.

**Boot splash.** Not a modal. The whole window shows the app mark centered at 80px. On Linux the custom title bar stays above it.

## Regions

**Workspace Settings rows.** Each row: directory text field, browse button, a depth stepper (left chevron, value, right chevron), and a remove button.

**Licenses groups.** `Assets`, `Frontend`, `Backend`, in that order; empty groups are omitted. Each row: package name, a license badge, and an external-link button when a URL exists.

**Boot splash.** One mark, white on dark and black on light, no text.

## Controls

**Workspace Settings**

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Directory field | placeholder `Select a directory...` | Edit the path | Enter saves the form |
| Browse | tooltip `Browse...` | System folder picker titled `Select Git Projects Directory` | none |
| Depth down | left chevron | Depth minus 1, minimum 1 | none |
| Depth up | right chevron | Depth plus 1, maximum 5 | none |
| Remove | tooltip `Remove` | Remove the row. Hidden when only one row remains | none |
| Add Directory | `Add Directory` | Append an empty row with depth 1 and focus it | none |
| Cancel | `Cancel` | Close without saving | Escape |
| OK | `OK` | Save the rows with a non-blank directory, then close | Enter |

Opened from the welcome screen (`Configure Workspace...`) and from Settings > Projects (`Configure...`).

**Open Source Licenses**

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| External link | link icon; the tooltip is the URL | Open in the system browser | none |
| Close | `Close` | Close | Escape, backdrop click |
| Help menu | `Open Source Licenses` | Open the dialog | none |

**Boot splash.** No controls.

## Copy

Workspace Settings:

- `Workspace Settings`
- `Add directories containing your Git repositories. The scan depth controls how many levels deep to search for projects within each directory.`
- `Select a directory...`
- `Browse...`
- `Select Git Projects Directory`
- `Remove`
- `Add Directory`
- `Cancel`, `OK`
- Launchers: `Configure Workspace...` (welcome), `Configure...` (settings, next to `Workspace Directories` with the placeholder `Not configured`)

Open Source Licenses:

- `Open Source Licenses`
- `Assets`, `Frontend`, `Backend`
- `Close`
- Badge: the license identifier, or `Unknown`

Boot splash: no text.

## Visual

Both dialogs share the clone-dialog look: sidebar background, subtle border, rounded corners, soft shadow, 16px padding, 14px semibold title. Description 12px muted. Directory rows: field 28 tall on the input background, browse 28 square in secondary colors, depth buttons 18 with a hover background and disabled at 30% opacity, value 11px centered, remove 22 at half opacity until hover. `Add Directory` is a borderless link-colored text button with a plus icon. Actions right-aligned: `OK` primary, `Cancel` secondary.

Licenses: group titles 11px bold uppercase muted; rows 26 tall with a hover background; badge 11px in a pill on the badge colors; the link button at half opacity until hover.

Boot splash: fixed colors independent of the app theme. Dark background `#1e1e1e` with the white mark; light background `#f3f3f3` with the black mark, chosen by the OS color scheme. Mark at 60% opacity. The window background starts in the same color so nothing flashes.

## States

**Workspace Settings.** Opens with one row per configured directory, or one blank row at depth 1 (Remove hidden). Depth 1 disables the down chevron; depth 5 disables the up chevron. Save drops blank rows; an all-blank form saves an empty list. Cancel, backdrop, and Escape discard edits. The first field is focused on open; a new row is focused when added.

**Open Source Licenses.** Always populated from the build: app dependencies for the frontend and backend, plus the bundled font (MesloLGS Nerd Font Mono, Apache-2.0, `https://github.com/ryanoasis/nerd-fonts`). Sorted by name, deduplicated. Rows without a URL have no link.

**Boot splash.** Visible from first paint until startup decides between the welcome screen and a repository. If a recent project is known, a placeholder of the repository chrome shows instead of the splash while it opens.

## Interactions

**Workspace Settings.** Saving rewrites the workspace list in settings and reruns the welcome scan.

**Open Source Licenses.** Help > Open Source Licenses opens it with or without a repository.

**Boot splash.** None.

## Keyboard

Workspace Settings: Escape closes without saving; Enter saves from anywhere in the dialog. Depth is mouse-only.

Open Source Licenses: Escape closes. No list navigation.

## Persistence

Workspace directories: app settings (`Projects`), each with a directory and a depth from 1 to 5.
