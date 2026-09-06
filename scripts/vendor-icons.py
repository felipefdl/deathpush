#!/usr/bin/env python3
"""Vendor the icon sets and regenerate the Material tree-icon lookup tables.

Chrome icons come from Lucide, one file per icon, named exactly as upstream.
Tree icons come from the Zed Material Icon Theme extension: the SVGs it
references plus their `_light` siblings, and a generated Rust module with the
file-name, suffix, and folder-name tables.

    ./scripts/vendor-icons.py
"""

from __future__ import annotations

import io
import json
import re
import shutil
import sys
import tarfile
import time
import urllib.request
from pathlib import Path

MATERIAL_REPO = "zed-extensions/material-icon-theme"
MATERIAL_REV = "5ec848638409e4578d9e8c8478041fcab1df15f8"
# The Zed extension repackages the VS Code theme; the icons keep the upstream license.
MATERIAL_UPSTREAM = "material-extensions/vscode-material-icon-theme"
LUCIDE_REPO = "lucide-icons/lucide"
LUCIDE_TAG = "1.41.0"

ROOT = Path(__file__).resolve().parent.parent
CHROME_DIR = ROOT / "assets" / "icons"
TREE_DIR = ROOT / "assets" / "material-icons"
GENERATED = ROOT / "crates" / "app" / "src" / "repo" / "explorer" / "material.rs"

# Chrome icons, by their upstream Lucide name. Every entry must be referenced by
# the app; unused icons do not belong in the binary.
LUCIDE_ICONS = [
  "archive",
  "arrow-down-up",
  "arrow-left-right",
  "arrow-up",
  "binary",
  "bookmark",
  "check",
  "chevron-down",
  "chevron-left",
  "chevron-right",
  "cloud",
  "cloud-download",
  "cloud-upload",
  "columns-2",
  "copy",
  "ellipsis",
  "external-link",
  "file",
  "file-archive",
  "file-code",
  "file-image",
  "file-braces",
  "file-plus",
  "file-text",
  "folder",
  "folder-git-2",
  "folder-open",
  "folder-plus",
  "git-branch",
  "git-commit-horizontal",
  "rotate-ccw-clock",
  "list",
  "list-tree",
  "maximize",
  "minimize",
  "minus",
  "pencil",
  "plus",
  "refresh-cw",
  "rows-2",
  "search",
  "tag",
  "terminal",
  "trash",
  "triangle-alert",
  "undo-2",
  "x",
]


def fetch(url: str, attempts: int = 4) -> bytes:
  """GET with a short backoff; raw.githubusercontent.com throttles bursts."""
  request = urllib.request.Request(url, headers={"User-Agent": "deathpush-vendor-icons"})
  for attempt in range(attempts):
    try:
      with urllib.request.urlopen(request) as response:
        return response.read()
    except urllib.error.HTTPError:
      raise
    except OSError:
      if attempt == attempts - 1:
        raise
      time.sleep(1 + attempt)
  raise AssertionError("unreachable")


def minify_svg(source: str) -> str:
  """Collapse Lucide's pretty-printed SVG and drop its CSS class."""
  svg = re.sub(r'\s+class="[^"]*"', "", source)
  svg = re.sub(r"\s*\n\s*", " ", svg)
  svg = re.sub(r"\s{2,}", " ", svg)
  svg = svg.replace("> <", "><").replace(" />", "/>").replace(" >", ">")
  return svg.strip() + "\n"


def vendor_lucide() -> None:
  base = f"https://raw.githubusercontent.com/{LUCIDE_REPO}/{LUCIDE_TAG}/icons"
  svgs = {name: minify_svg(fetch(f"{base}/{name}.svg").decode()) for name in LUCIDE_ICONS}
  shutil.rmtree(CHROME_DIR, ignore_errors=True)
  CHROME_DIR.mkdir(parents=True)
  for name, svg in svgs.items():
    (CHROME_DIR / f"{name}.svg").write_text(svg)
  print(f"chrome: {len(svgs)} Lucide icons -> {CHROME_DIR.relative_to(ROOT)}")


def download_material() -> dict[str, bytes]:
  url = f"https://codeload.github.com/{MATERIAL_REPO}/tar.gz/{MATERIAL_REV}"
  archive = tarfile.open(fileobj=io.BytesIO(fetch(url)), mode="r:gz")
  files: dict[str, bytes] = {}
  for member in archive.getmembers():
    if not member.isfile():
      continue
    name = member.name.split("/", 1)[1]
    if name.startswith("icons/") or name == "icon_themes/material-icon-theme.json":
      handle = archive.extractfile(member)
      if handle is not None:
        files[name] = handle.read()
  return files


def rust_string(value: str) -> str:
  return '"' + value.replace("\\", "\\\\").replace('"', '\\"') + '"'


def vendor_material() -> None:
  files = download_material()
  theme = json.loads(files["icon_themes/material-icon-theme.json"])["themes"][0]
  file_icons = theme["file_icons"]
  directory = theme["directory_icons"]

  def asset(path: str) -> str:
    return path.removeprefix("./icons/")

  def by_type(key: str) -> str | None:
    """Icon file for a type key. A few keys (`git`, `template`) are missing from the
    upstream `file_icons` map even though the SVG ships; use the same-named file."""
    entry = file_icons.get(key)
    if isinstance(entry, dict):
      return asset(entry["path"])
    return f"{key}.svg" if f"icons/{key}.svg" in files else None

  # Interned icon table: index -> (dark asset, light asset).
  order: list[str] = []
  index: dict[str, int] = {}

  def intern(name: str) -> int:
    if name not in index:
      index[name] = len(order)
      order.append(name)
    return index[name]

  def lower_unique(mapping: dict[str, object]) -> dict[str, object]:
    out: dict[str, object] = {}
    for key, value in mapping.items():
      lowered = key.lower()
      if not lowered.isascii():
        continue
      out[lowered] = value
    return out

  stems: list[tuple[str, int]] = []
  for name, key in sorted(lower_unique(theme["file_stems"]).items()):
    icon = by_type(str(key))
    if icon:
      stems.append((name, intern(icon)))

  suffixes: list[tuple[str, int]] = []
  for suffix, key in sorted(lower_unique(theme["file_suffixes"]).items()):
    icon = by_type(str(key))
    if icon:
      suffixes.append((suffix, intern(icon)))

  folders: list[tuple[str, int, int]] = []
  for name, pair in sorted(lower_unique(theme["named_directory_icons"]).items()):
    if not isinstance(pair, dict):
      continue
    folders.append((name, intern(asset(pair["collapsed"])), intern(asset(pair["expanded"]))))

  default_file = intern(by_type("file") or "file.svg")
  default_folder = intern(asset(directory["collapsed"]))
  default_folder_open = intern(asset(directory["expanded"]))

  # Write the SVGs, pairing each icon with its `_light` sibling when one exists.
  shutil.rmtree(TREE_DIR, ignore_errors=True)
  TREE_DIR.mkdir(parents=True)
  pairs: list[tuple[str, str]] = []
  for name in order:
    light = name.replace(".svg", "_light.svg")
    if f"icons/{light}" not in files:
      light = name
    for asset_name in {name, light}:
      (TREE_DIR / asset_name).write_bytes(files[f"icons/{asset_name}"])
    pairs.append((name, light))
  license_url = f"https://raw.githubusercontent.com/{MATERIAL_UPSTREAM}/main/LICENSE"
  (TREE_DIR / "LICENSE").write_bytes(fetch(license_url))

  lines: list[str] = [
    "//! Material Icon Theme lookup tables.",
    "//!",
    f"//! Generated by `scripts/vendor-icons.py` from {MATERIAL_REPO}",
    f"//! at `{MATERIAL_REV[:12]}`. Do not edit by hand.",
    "",
    "use std::cmp::Ordering;",
    "",
    "/// Asset paths as `(dark, light)`; the light slot repeats the dark one when the set has no variant.",
    "static ICONS: &[(&str, &str)] = &[",
  ]
  for dark, light in pairs:
    lines.append(f"  ({rust_string(f'material-icons/{dark}')}, {rust_string(f'material-icons/{light}')}),")
  lines += ["];", "", "/// Whole file names, lowercased and sorted.", "static STEMS: &[(&str, u16)] = &["]
  for name, icon in stems:
    lines.append(f"  ({rust_string(name)}, {icon}),")
  lines += ["];", "", "/// File extensions, lowercased and sorted; keys may hold a dot.", "static SUFFIXES: &[(&str, u16)] = &["]
  for suffix, icon in suffixes:
    lines.append(f"  ({rust_string(suffix)}, {icon}),")
  lines += [
    "];",
    "",
    "/// Directory names, lowercased and sorted, as `(name, collapsed, expanded)`.",
    "static FOLDERS: &[(&str, u16, u16)] = &[",
  ]
  for name, collapsed, expanded in folders:
    lines.append(f"  ({rust_string(name)}, {collapsed}, {expanded}),")
  lines += [
    "];",
    "",
    f"const DEFAULT_FILE: u16 = {default_file};",
    f"const DEFAULT_FOLDER: u16 = {default_folder};",
    f"const DEFAULT_FOLDER_OPEN: u16 = {default_folder_open};",
    "",
    "fn asset(icon: u16, light: bool) -> &'static str {",
    "  let (dark, bright) = ICONS[icon as usize];",
    "  if light { bright } else { dark }",
    "}",
    "",
    "/// Orders a lowercase ASCII key against an arbitrary-case probe without allocating.",
    "fn cmp_key(key: &str, probe: &str) -> Ordering {",
    "  let key = key.as_bytes();",
    "  let probe = probe.as_bytes();",
    "  for (a, b) in key.iter().zip(probe) {",
    "    match a.cmp(&b.to_ascii_lowercase()) {",
    "      Ordering::Equal => (),",
    "      other => return other,",
    "    }",
    "  }",
    "  key.len().cmp(&probe.len())",
    "}",
    "",
    "fn lookup(table: &[(&str, u16)], probe: &str) -> Option<u16> {",
    "  table",
    "    .binary_search_by(|(key, _)| cmp_key(key, probe))",
    "    .ok()",
    "    .map(|at| table[at].1)",
    "}",
    "",
    "/// Icon for a file name: whole name first, then the longest matching suffix.",
    "pub fn file(name: &str, light: bool) -> &'static str {",
    "  if let Some(icon) = lookup(STEMS, name) {",
    "    return asset(icon, light);",
    "  }",
    "  let mut rest = name;",
    "  while let Some((_, tail)) = rest.split_once('.') {",
    "    if let Some(icon) = lookup(SUFFIXES, tail) {",
    "      return asset(icon, light);",
    "    }",
    "    rest = tail;",
    "  }",
    "  asset(DEFAULT_FILE, light)",
    "}",
    "",
    "/// Icon for a directory name, open or closed.",
    "pub fn folder(name: &str, expanded: bool, light: bool) -> &'static str {",
    "  let icon = FOLDERS",
    "    .binary_search_by(|(key, _, _)| cmp_key(key, name))",
    "    .ok()",
    "    .map(|at| {",
    "      let (_, collapsed, opened) = FOLDERS[at];",
    "      if expanded { opened } else { collapsed }",
    "    })",
    "    .unwrap_or(if expanded { DEFAULT_FOLDER_OPEN } else { DEFAULT_FOLDER });",
    "  asset(icon, light)",
    "}",
    "",
  ]
  GENERATED.write_text("\n".join(lines))
  svgs = len(list(TREE_DIR.glob("*.svg")))
  print(f"tree:   {svgs} Material icons -> {TREE_DIR.relative_to(ROOT)}")
  print(f"tables: {len(stems)} names, {len(suffixes)} suffixes, {len(folders)} folders -> {GENERATED.relative_to(ROOT)}")


def main() -> int:
  vendor_lucide()
  vendor_material()
  print("run `just fmt` to format the generated module")
  return 0


if __name__ == "__main__":
  sys.exit(main())
