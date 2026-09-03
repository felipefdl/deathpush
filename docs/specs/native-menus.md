# Native menus

Status: Current product
Date: 2026-09-02

## Purpose

The OS-level command surface. On macOS it is the system menu bar; on Windows the in-window menu bar; on Linux the same menu flattened into one dropdown behind a menu button in the custom title bar. See the [app shell](app-shell.md) for the Linux title bar itself.

## Layout

Top-level menus, left to right: `DeathPush`, `File`, `Edit`, `View`, `Git`, `Terminal`, `Window`, `Help`.

Linux: no visible menu bar. One dropdown under the menu button holds a flattened subset (see the Linux table).

## Regions

**DeathPush.** About, Settings, Install Command Line Tool (macOS and Windows), then on macOS Services, Hide, Hide Others, Show All, then Quit (`Exit` on Windows).

**File.** New Window, Open Repository, Clone Repository, Close Window.

**Edit.** Undo, Redo, Cut, Copy, Paste, Select All. Standard OS editing commands.

**View.** Quick Open, Changes, History, Toggle Diff Mode, Color Theme, Zoom In, Zoom Out, Reset Zoom. Development builds add Inspect Element.

**Git.** Pull, Push, Fetch, Stage All, Unstage All, Stash, Stash Pop, Undo Last Commit.

**Terminal.** New Terminal, Kill Terminal, Toggle Terminal.

**Window.** Minimize, Maximize (`Zoom` on macOS), Close Window.

**Help.** Open Source Licenses.

## Controls

Shortcuts use Cmd on macOS and Ctrl elsewhere.

### DeathPush

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| About | `About {appName}` on macOS (`About deathpush`, after the executable name); `About` on Windows | System About panel | none |
| Settings | `Settings...` | Show [Settings](settings.md) in the main panel | Cmd/Ctrl+, |
| Install Command Line Tool | `Install Command Line Tool...` | Install or uninstall the `dp` and `deathpush` commands (see Interactions). Not on Linux | none |
| Services | `Services` | macOS Services submenu | none |
| Hide | `Hide {appName}` | Hide the app (macOS) | Cmd+H |
| Hide Others | `Hide Others` | macOS | Alt+Cmd+H |
| Show All | `Show All` | macOS | none |
| Quit | `Quit {appName}` (macOS), `Exit` (Windows), `Quit` (Linux) | Quit | Cmd+Q on macOS |

### File

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| New Window | `New Window` | Open a new window on the welcome screen | Cmd/Ctrl+N |
| Open Repository | `Open Repository...` | Folder picker titled `Open Git Repository`, then open | Cmd/Ctrl+O |
| Clone Repository | `Clone Repository...` | [Clone dialog](clone-dialog.md) | none |
| Close Window | `Close Window` (`Close` on Windows) | Close, after the terminal-process check | Cmd+W on macOS, Alt+F4 on Windows |

### Edit

Undo (Cmd/Ctrl+Z), Redo (Cmd+Shift+Z on macOS, Ctrl+Y elsewhere), Cut (Cmd/Ctrl+X), Copy (Cmd/Ctrl+C), Paste (Cmd/Ctrl+V), Select All (Cmd/Ctrl+A). OS behavior; the app adds nothing.

### View

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Quick Open | `Quick Open...` | [Quick Open](quick-open.md) | Cmd/Ctrl+P |
| Changes | `Changes` | SCM sidebar and diff panel | Cmd/Ctrl+1 |
| History | `History` | [History](history.md) | Cmd/Ctrl+Shift+2 |
| Toggle Diff Mode | `Toggle Diff Mode` | Flip the Diff Layout setting | Cmd/Ctrl+Shift+P |
| Color Theme | `Color Theme...` | [Theme picker](theme-picker.md) | none |
| Zoom In | `Zoom In` | Zoom level plus 1, maximum 9 | Cmd/Ctrl+= |
| Zoom Out | `Zoom Out` | Zoom level minus 1, minimum -5 | Cmd/Ctrl+- |
| Reset Zoom | `Reset Zoom` | Zoom level 0 | Cmd/Ctrl+0 |
| Inspect Element | `Inspect Element` | Developer tools (development builds only) | Cmd/Ctrl+Shift+I |

### Git

No shortcuts.

| Control | Copy | Action |
|---|---|---|
| Pull | `Pull` | Pull with merge |
| Push | `Push` | Push |
| Fetch | `Fetch` | Fetch and prune |
| Stage All | `Stage All` | Stage everything |
| Unstage All | `Unstage All` | Unstage everything |
| Stash | `Stash...` | Stash tracked changes with no message. No prompt despite the ellipsis |
| Stash Pop | `Stash Pop` | Pop the newest stash |
| Undo Last Commit | `Undo Last Commit` | Confirm, then move the last commit back to the index |

### Terminal

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| New Terminal | `New Terminal` | New terminal group; show the panel on the Terminal tab | Cmd/Ctrl+Shift+J |
| Kill Terminal | `Kill Terminal` | Kill the active group | none |
| Toggle Terminal | `Toggle Terminal` | Show or hide the panel | Cmd/Ctrl+J |

### Window

Minimize (`Minimize`, Cmd/Ctrl+M), Maximize (`Zoom` on macOS, `Maximize` elsewhere), Close Window (as in File).

### Help

| Control | Copy | Action |
|---|---|---|
| Open Source Licenses | `Open Source Licenses` | Licenses overlay. See [overlays](overlays.md) |

### Linux dropdown

In order, with separators between groups. Rows marked repo-only are disabled when no repository is open.

| Copy | Shortcut label | Repo-only |
|---|---|---|
| `New Window` | `Ctrl+N` | no |
| `Open Repository...` | `Ctrl+O` | no |
| `Clone Repository...` | | no |
| `Changes` | `Ctrl+1` | yes |
| `History` | `Ctrl+Shift+2` | yes |
| `Toggle Diff Mode` | `Ctrl+Shift+P` | yes |
| `Color Theme...` | | no |
| `Zoom In` | `Ctrl+=` | no |
| `Zoom Out` | `Ctrl+-` | no |
| `Reset Zoom` | `Ctrl+0` | no |
| `Pull` | | yes |
| `Push` | | yes |
| `Fetch` | | yes |
| `Stage All` | | yes |
| `Unstage All` | | yes |
| `Stash...` | | yes |
| `Stash Pop` | | yes |
| `Undo Last Commit` | | yes |
| `New Terminal` | `Ctrl+Shift+J` | yes |
| `Kill Terminal` | | yes |
| `Toggle Terminal` | `Ctrl+J` | yes |
| `Settings...` | `Ctrl+,` | no |
| `Quit` | | no |

The Linux dropdown omits About, Services, Hide, Edit, Quick Open, Inspect Element, Help, Close Window, and Install Command Line Tool.

## Copy

Menu titles: `DeathPush`, `File`, `Edit`, `View`, `Git`, `Terminal`, `Window`, `Help`.

The app name in About, Hide, and Quit is the executable name (`deathpush`), so macOS shows `About deathpush`, `Hide deathpush`, and `Quit deathpush` while the menu title is `DeathPush`.

Folder picker title: `Open Git Repository`.

Command line tool dialogs (system dialogs):

- Install confirmation: title `Install Command Line Tool`, warning style, buttons `Install` and `Cancel`. Message:

  `Install dp and deathpush commands to /usr/local/bin so you can open repositories from any terminal.`

  `Examples:`

  `  dp .`

  `  deathpush ~/projects/my-repo`

- Install success: title `Command Line Tool`, message `Commands dp and deathpush installed successfully. Restart your terminal to start using them.`
- Already installed: title `Command Line Tool`, warning style, buttons `Uninstall` and `Cancel`, message `Command line tools 'dp' and 'deathpush' are already installed. Would you like to uninstall them?`
- Uninstall success: title `Command Line Tool`, message `Command line tools have been uninstalled.`

Undo Last Commit confirmation: title `Confirm`, warning style, buttons `Continue` and `Cancel`, message `Undo last commit? Changes will be moved back to staging.`

## Visual

Native menus use the OS menu chrome. The Linux dropdown: at least 260 wide, 12px corner radius, 6px padding, on the menu colors; items 13px with 8px by 16px padding and 8px corners, the shortcut label 12px at 60% opacity on the right; hover uses the menu selection colors; disabled at 40% opacity; hairline separators.

The native menu bar follows the app theme (dark or light) on macOS and Windows.

## States

**No repository.** Settings, Changes, History, Toggle Diff Mode, every Git item, the Terminal items, and Quick Open are disabled. Everything else stays enabled. On Linux, Settings stays enabled.

**Repository open.** Those items enable. The state follows the focused window: focusing a welcome window disables them again.

**Development build.** Inspect Element is present. Release builds omit it.

**Command line tool installed.** The same menu item offers uninstall instead of install.

**Elevation cancelled.** If the user cancels the OS authorization prompt during install, nothing happens and no error shows.

**Git errors.** Failed Git items show the app toast.

**Stash with nothing to stash.** No guard; the Git error shows as a toast.

## Interactions

**Open Repository.** Folder picker; cancel does nothing.

**Settings.** The menu item always shows Settings. The Cmd/Ctrl+, shortcut toggles between Settings and Changes.

**Command line tool.** Installs `dp` and `deathpush` launchers so that `dp /path/to/repo` opens that repository. On macOS they are links in `/usr/local/bin`, installed after an administrator prompt. On Windows they are scripts in the user's local app data folder. The dialog text names `/usr/local/bin` on both.

**Quit and last window.** On macOS closing the last window keeps the app running; reopening the app from the Dock creates a new window.

**Fetch.** Always prunes, matching the SCM overflow Fetch.

## Keyboard

Shortcuts as listed. The window also handles Cmd/Ctrl+,, Cmd/Ctrl+O, Cmd/Ctrl+P, Cmd/Ctrl+Shift+P, the zoom keys, Cmd/Ctrl+J, and Cmd/Ctrl+1 itself, so they work whether or not the menu is visible. Cmd/Ctrl+2 opens Explorer and is not a menu item; History is Cmd/Ctrl+Shift+2. Cmd/Ctrl+3 focuses the terminal and has no menu item. The theme picker chord Cmd/Ctrl+K Cmd/Ctrl+T is not shown on the menu item.

## Persistence

Zoom level and diff layout persist app-wide.
