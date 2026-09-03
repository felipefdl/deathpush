# Clone Repository dialog

Status: Current product
Date: 2026-09-02

## Purpose

A modal for cloning a remote into a local parent folder, then opening the clone in the current window. It appears over the [welcome screen](welcome-screen.md) or the [app shell](app-shell.md).

## Layout

Full-window transparent backdrop. The dialog is 440 wide, horizontally centered, 60 from the top. Inside, top to bottom: title, Repository URL field, Directory field with a browse button, then a right-aligned action row with `Cancel` and `Clone`.

There is no branch field. The clone checks out the remote's default branch.

## Regions

**Backdrop.** Click closes the dialog.

**Title.** `Clone Repository`.

**URL field.** Label and a single-line text field, focused on open.

**Directory field.** Label, text field, and a square browse button with a folder icon.

**Actions.** `Cancel`, then `Clone`.

Errors are not shown inside the dialog. They go to the app toast, which renders above the dialog.

## Controls

| Control | Copy | Action | Shortcut |
|---|---|---|---|
| Backdrop | none | Close | none |
| URL field | placeholder `https://github.com/user/repo.git` | The remote URL | none |
| Directory field | placeholder `Select a directory...` | The parent folder | none |
| Browse | folder icon | System folder picker titled `Choose directory to clone into`; fills Directory | none |
| Cancel | `Cancel` | Close. Disabled while cloning | Escape (works even while cloning) |
| Clone | `Clone`, `Cloning...` while running | Start the clone. Disabled when either field is blank after trimming, or while cloning | Enter |
| File menu | `Clone Repository...` | Open this dialog | none |
| Welcome button | `Clone Repository` | Open this dialog | none |
| SCM overflow | `Clone Repository...` | Open this dialog | none |

## Copy

- `Clone Repository`
- `Repository URL`
- `https://github.com/user/repo.git`
- `Directory`
- `Select a directory...`
- `Choose directory to clone into`
- `Cancel`
- `Clone`
- `Cloning...`

Toast messages on failure: `Git CLI failed: ` followed by the Git error output, or `Git is not installed. Please install git and try again.`

## Visual

Dialog on the sidebar background with a subtle border, rounded corners, a soft shadow, and 16px padding. Title 14px semibold with 12px below. Fields 28 tall on the input background with the focus border when focused; labels 12px with 4px below. Browse button 28 square in secondary button colors. Actions right-aligned, 8px apart, 26 tall; `Clone` in primary button colors, `Cancel` in secondary; disabled at half opacity. Fields allow text selection and have autocomplete, autocorrect, autocapitalize, and spellcheck off.

## States

**Empty.** Both fields empty; Clone disabled.

**Ready.** Both non-blank; Clone enabled.

**Cloning.** Label `Cloning...`; Clone and Cancel disabled; a backdrop click and Escape still close the dialog, and the clone continues in the background.

**Success.** The clone opens in this window, is added to recents, and the dialog closes.

**Failure.** The toast shows the error; the dialog stays open and returns to Ready.

## Interactions

**Open.** The welcome button, the File menu, the Linux menu, or the SCM overflow.

**Browse.** Folder picker; cancelling leaves the field unchanged.

**Clone.** The target is the Directory joined with the repository name taken from the URL (last path segment, `.git` stripped; `repo` if empty). On success the window title becomes `{repoName} ({branch}) - DeathPush`, or `{repoName} - DeathPush` when detached.

**Dismiss.** Cancel, backdrop, or Escape. No confirmation.

**Output.** The clone command is logged in the terminal Output tab.

## Keyboard

Inside the dialog: Escape closes; Enter clones (same guards as the button); Tab moves through URL, Directory, Browse, Cancel, Clone. No focus trap.

## Persistence

None beyond the recents entry on success.
