# 2. Zed theme schema

Status: Accepted
Date: 2026-09-06
Author: Felipe Lima

## Context

The old theme pipeline ingested 65 tm-themes files in the VS Code format. Each file supplied `colors` and `tokenColors`; `crates/core/src/theme/palette.rs` derived a 68-role `UiPalette` through 54 `unwrap_or` fallback cascades. VS Code themes only declared the keys they override: `dark-plus` declares 28 chrome keys, `light-plus` 33, `vesper` 71, and `min-dark` 84, with a median around 200. VS Code supplies roughly 600 defaults that DeathPush never had. Sparse themes therefore rendered from computed guesses, and opening the theme was the only way to catch a bad result.

Forces:

- The app needs a complete editor-shaped color contract instead of guesses spread across fallback cascades.
- Syntax highlighting already uses tree-sitter captures, so the theme format should name those captures directly.
- A theme family should be able to ship several appearances in one vendored file, while users keep a folder for themes outside the bundled catalog.

Options considered:

- Layer VS Code's default color table under the existing parser. Rejected: it keeps the sparse format and its implicit defaults.
- Rename `UiPalette` fields to Zed key names. Rejected: it requires a 293-site rewrite with no behavior gain.
- Read a generic key map at paint time. Rejected: it gives no compile-time guarantee that the app has every role it needs.
- Reject incomplete themes. Rejected: Warm Burnout declares 119 UI keys and omits 25 keys that One declares, so the rule would refuse the default theme.

## Decision

Zed's theme schema is the only ingest format. Zed themes declare a complete, editor-shaped key set: Zed's One declares 140 UI keys and 46 syntax keys, while Catppuccin's port declares 174 UI keys and 101 syntax keys. Syntax keys are already tree-sitter capture names, so the 171-line longest-prefix scope matcher in `theme/syntax.rs` becomes a name lookup.

`UiPalette` keeps its existing role names and is filled 1:1 from Zed keys. The 293 `palette.<field>` call sites therefore stay unchanged.

Missing keys resolve through an alias chain inside the same theme first: `version_control.added` -> `created`, `text.placeholder` -> `text.muted`, and `terminal.ansi.dim_*` -> the matching normal slot. If the alias chain has no value, resolution falls back to the base theme with the same appearance, One Dark or One Light. Warm Burnout declares 119 UI keys and omits 25 keys that One declares, including `version_control.*`; falling straight back to One would put cool greens and reds inside a warm palette.

## Consequences

Easier: the bundled themes have a checkable key contract, syntax mapping is direct, one family file can carry several themes, and a unit test can check completeness instead of someone opening every theme by eye.

Harder:

- The bundled catalog drops from 65 themes to 13: One has 2, Ayu 3, Gruvbox 6, and Warm Burnout 2. All bundled themes are MIT licensed.
- Themes outside the bundled catalog live under the user themes folder at `<config_dir>/deathpush/themes/*.json`.
- Defaults become Warm Burnout Dark and Warm Burnout Light.
- Theme ids are slugs of the authored theme name. Old ids have no migration or aliases; an unknown id falls back to the appearance default.
- The parser and resolver now commit to Zed's schema and its alias and base-theme rules.
