# 3. Folders without a Git repository open as a plain explorer

Status: Accepted
Date: 2026-09-07
Author: Felipe Lima

## Context

Every open path used to resolve to a Git repository. A folder outside any repository was refused, so the welcome screen was the only state where no repository was open. That refusal hit real work: a scratch folder, a freshly created project, notes next to a checkout, or `dp .` in the wrong directory. The parts of the app that do not need Git — the file tree, the file viewer with autosave, the terminal, Quick Open, and Settings — were already root-relative and worked on any directory.

Two other shapes were considered. Refusing the folder with an explicit error is honest but leaves the user with nothing to do in the app. Prompting to run `git init` at open time asks a question before the user has looked at the folder, and answering no leaves no window to land in.

## Decision

Opening a folder that is not inside any Git repository mounts the normal app shell on that folder. The runtime root is the folder itself, and a folder inside a repository still opens the enclosing repository. `SessionRepo` and `RepositoryStatus` carry `has_repository`, and every Git surface is gated on that one flag: the sidebar lands on Explorer, the status bar drops its branch, sync, blame, and last-commit items, the diff and History panels read `Diffs need a Git repository` and `Commit history needs a Git repository`, `Add to .gitignore` disappears from the Explorer menu, and menu items split into folder-gated and Git-gated sets. Core refuses a Git intent in such a folder with `<path> is not a Git repository`.

The Changes tab carries the affordance instead of a prompt: `This folder is not a Git repository` with an `Initialize Repository` button that runs `Intent::InitRepository`, after which the shell refreshes in place as a repository with an unborn HEAD.

The surfaces are specified in [app shell](../specs/app-shell.md), [SCM Changes](../specs/scm-changes.md), and [native menus](../specs/native-menus.md).

## Consequences

Easier: any folder opens, recents hold plain folders, and starting a repository is one click from inside the folder the user is already looking at.

Harder:

- Every Git-touching surface needs a repository-less branch, and a new one must pick a gate instead of assuming a repository.
- The Explorer tree has two sources, Git status and a filesystem walk, so it caps the walk at 50,000 files.
- The window can change from folder to repository while open, so views refresh in place rather than assuming the root's Git state is fixed for the session's life.
