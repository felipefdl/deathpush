# Settings

Status: Current product
Date: 2026-09-02

## Purpose

App settings shown in the main panel of a repository window. The sidebar stays on Changes. Every change applies at once and persists app-wide. Git identity (user name and email) reads and writes the Git configuration instead. Reset restores every app setting after a system confirmation.

## Layout

Header row: `Settings` on the left, `Reset to Defaults` on the right. Below, a scrollable stack of sections in this order: Appearance, Editor, Diff Viewer, Git, Projects, Terminal.

Terminal has subsections: General, Text & Font, Cursor, Scrolling, Behavior, Rendering, Shell.

Each row is a label on the left and its control on the right. Section and subsection titles render uppercase.

## Regions

**Header.** Title and reset button.

**Appearance.** Theme, tree, sidebar, UI font, zoom, terminal on start.

**Editor.** File viewer and diff editor font and wrap.

**Diff Viewer.** Diff rendering options.

**Git.** Blame toggle and identity fields.

**Projects.** Read-only summary of workspace directories plus `Configure...`.

**Terminal.** Terminal appearance, behavior, and shell.

## Controls

| Control | Copy | Type | Default |
|---|---|---|---|
| Reset to Defaults | `Reset to Defaults` | Button, then confirmation | n/a |
| Color Theme | current theme label plus the hint `Cmd+K Cmd+T` | Full-width button; opens the [theme picker](theme-picker.md) | Warm Burnout Dark |
| Preferred Dark Theme | `Preferred Dark Theme` | Select of dark themes | Warm Burnout Dark |
| Preferred Light Theme | `Preferred Light Theme` | Select of light themes | Warm Burnout Light |
| Tree Density | `Tree Density` | Select: Compact / Default / Relaxed | Compact |
| Tree Icons | `Tree Icons` | Select: Minimal / Standard / Complete | Complete |
| Sidebar Position | `Sidebar Position` | Select: Left / Right | Left |
| UI Font Family | `UI Font Family` | Text | the system UI font |
| UI Font Size | `UI Font Size` | Number 10 to 20 | 13 |
| Zoom | `Zoom` | Select of percentages (1.2 ^ level, level -5 to 9) | 100% |
| Always Open Terminal on Start | `Always Open Terminal on Start` | Toggle | off |
| Editor Font Size | `Font Size` | Number 8 to 32 | 13 |
| Editor Font Family | `Font Family` | Text | `MesloLGS Nerd Font Mono`, then Menlo, Monaco, Courier New, monospace |
| Line Height | `Line Height` | Number 10 to 60 | 20 |
| Tab Size | `Tab Size` | Number 1 to 8 | 4 |
| Word Wrap | `Word Wrap` | Select: Off / On | Off |
| Diff Layout | `Diff Layout` | Select: Side by Side / Inline | Side by Side |
| Inline Hunk Actions | `Inline Hunk Actions` | Toggle | off |
| Line Numbers | `Line Numbers` | Toggle | on |
| Diff Indicators | `Diff Indicators` | Select: None / Bars / Classic (+/−) | None |
| Inline Changes | `Inline Changes` | Select: Smart Words / Words / Characters / None | Smart Words |
| Background Highlighting | `Background Highlighting` | Toggle | on |
| Hunk Separators | `Hunk Separators` | Select: Compact Line Info / Line Info / Metadata / Simple | Simple |
| Git Blame | `Git Blame` | Toggle | on |
| User Name | `User Name` | Text, saved 500 ms after typing stops | from Git config |
| User Email | `User Email` | Text, saved 500 ms after typing stops | from Git config |
| Workspace Directories | `Workspace Directories`, placeholder `Not configured` | Read-only text plus a `Configure...` button | none |
| Terminal Font Size | `Font Size` | Number 8 to 32 | 13 |
| Terminal Font Family | `Font Family` | Text | the same stack as the editor |
| Terminal Line Height | `Line Height` | Number 0.8 to 3, step 0.1 | 1.2 |
| Font Weight | `Font Weight` | Select of weights | Normal |
| Font Weight Bold | `Font Weight Bold` | Select of weights | Bold |
| Letter Spacing | `Letter Spacing` | Number -5 to 10 | 0 |
| Cursor Style | `Cursor Style` | Select: Block / Underline / Bar | Block |
| Cursor Blink | `Cursor Blink` | Toggle | on |
| Cursor Width | `Cursor Width` | Number 1 to 5 | 1 |
| Cursor Inactive Style | `Cursor Inactive Style` | Select: Outline / Block / Bar / Underline / None | Outline |
| Scrollback | `Scrollback for New Terminals (KiB)` | Number 500 to 100000, step 500 | 5000 |
| Copy on Select | `Copy on Select` | Toggle | off |
| Right Click Selects Word | `Right Click Selects Word` | Toggle | off |
| macOS Option Click Forces Selection | `macOS Option Click Forces Selection` | Toggle | off |
| Color Saturation | `Color Saturation` | Number 0.5 to 2, step 0.01 | 1.42 |
| Shell Path | `Shell Path` | Preset select plus an optional custom path | `Default ($SHELL)` |
| Bell Style | `Bell Style` | Select: Off / Sound / Visual / Both | Off |

Reset confirmation: system dialog, warning style, title `Reset to Defaults`, message `Reset all settings to defaults? This cannot be undone.`, buttons `Reset` and `Cancel`.

## Copy

Sections: `Appearance`, `Editor`, `Diff Viewer`, `Git`, `Projects`, `Terminal`. Subsections: `General`, `Text & Font`, `Cursor`, `Scrolling`, `Behavior`, `Rendering`, `Shell`.

Shell presets: `Default ($SHELL)`, the shells common to the platform, and a custom entry that reveals a path field.

`Classic (+/−)` uses the Unicode minus sign.

## Visual

Controls are right-aligned. Filled selects. Pill toggles. Number fields with stepper buttons. `Reset to Defaults` is an outline button. Color Theme is a full-width button with the shortcut hint at its right edge. Base UI text is 13px.

## States

**Default.** The values in the table.

**Workspaces configured.** The read-only field lists each directory, or `directory:depth` when the depth is above 1, joined by `, `.

**Reset.** After confirmation every app setting returns to its default. Git identity is not an app setting and is left alone.

**Toggle from the keyboard.** Cmd/Ctrl+, opens Settings; pressing it again while on Settings returns to Changes.

## Interactions

**Live apply.** Every change persists at once and takes effect without a reload: diff rendering, trees, terminal, fonts, zoom.

**Color Theme.** Opens the theme picker.

**Configure...** Opens Workspace Settings. See [overlays](overlays.md).

**Git identity.** Read from the Git configuration when the page opens; written back 500 ms after the user stops typing.

## Keyboard

- Cmd/Ctrl+,: toggle Settings
- Cmd/Ctrl+K then Cmd/Ctrl+T: theme picker
- Tab moves through the controls in section order

## Persistence

All app settings persist app-wide on the machine, including the preferred dark and light themes. Git identity lives in the Git configuration.
