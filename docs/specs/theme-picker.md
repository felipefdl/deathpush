# Theme picker

Status: Current product
Date: 2026-09-02

## Purpose

A command-palette overlay for choosing the color theme. A theme colors the whole app: chrome, editors, diffs, trees, and the terminal. The bundled catalog contains 13 themes across the One, Ayu, Gruvbox, and Warm Burnout families, each tagged dark or light and carrying an authored label. User themes come from `<config_dir>/deathpush/themes/*.json` and are picked up when the picker opens. Defaults: Warm Burnout Dark for dark, Warm Burnout Light for light.

## Layout

A centered panel near the top of the window: a search field, then a scrollable list grouped into `dark themes` and `light themes`. When the OS prefers dark, the dark group comes first; otherwise light comes first. The first group label sits flush on the list; later group labels get a separator above them. The current theme row is highlighted. There are no checkmarks.

Clicking outside the panel cancels and restores the theme that was active when the picker opened.

## Regions

**Search.** Placeholder `Select Color Theme`. Focused on open.

**Group label.** `dark themes` / `light themes`.

**Rows.** The theme label. The active row (from keyboard or hover) is highlighted with the selection color.

## Controls

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Search | placeholder `Select Color Theme` | Filter themes by label substring | none |
| Row | theme label | Apply that theme and close | Enter on the active row |
| Backdrop | none | Cancel and restore the original theme | click |
| Settings Color Theme | current label plus `Cmd+K Cmd+T` | Open this picker | Cmd/Ctrl+K then Cmd/Ctrl+T |
| View menu Color Theme... | `Color Theme...` | Open this picker | none |

## Copy

- `Select Color Theme`
- `dark themes`
- `light themes`
- Theme labels: the authored theme name

## Visual

Panel 500 wide, up to 440 tall, on the sidebar background with a subtle border, rounded corners, and a soft shadow. The backdrop has no fill. The active row uses the list selection colors.

## States

**Open.** Search focused and empty; the active row is the current theme.

**Filtering.** Groups rebuild from the matches; the active row resets to the first match.

**Keyboard preview.** Up and Down move the active row and apply that theme at once as a preview. Hover moves the highlight but does not preview.

**Confirm.** Enter or click closes the picker, applies the theme, and records it as the preferred dark or light theme according to its kind.

**Cancel.** Escape on an empty query restores the original theme. A backdrop click cancels and restores in one step.

**Empty filter.** No rows; Enter does nothing.

## Interactions

**Open.** Cmd/Ctrl+K then Cmd/Ctrl+T (the chord expires after 1.5 s), the Settings Color Theme button, View > Color Theme..., or the Linux menu.

**Catalog refresh.** Opening the picker rescans the user themes folder. Bundled theme ids win collisions with user theme ids.

**Preferred themes.** Settings holds a preferred dark theme and a preferred light theme. When the OS color scheme flips, the app switches to the matching preferred theme. Picking a theme here also updates the preferred theme of that kind.

**Apply.** The theme's editor colors drive the chrome colors, the syntax highlighting, the tree styling, and the terminal palette.

## Keyboard

- Cmd/Ctrl+K then Cmd/Ctrl+T: open
- Up / Down: move and preview
- Enter: confirm
- Escape: clear a non-empty query; on an empty query, cancel and restore the original theme

## Persistence

The current theme, the preferred dark theme, and the preferred light theme persist app-wide.
