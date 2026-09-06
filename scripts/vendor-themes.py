#!/usr/bin/env python3
"""Vendor the bundled Zed theme families.

The bundled themes come from pinned upstream revisions so the app can embed a
small, reproducible catalog without carrying the full source repositories.

    ./scripts/vendor-themes.py
"""

from __future__ import annotations

import json
import shutil
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ZED_REPO = "zed-industries/zed"
ZED_REV = "5a9b9558db01a6b906cec2fb70a797affdc58cdd"
WARM_BURNOUT_REPO = "felipefdl/warm-burnout"
WARM_BURNOUT_REV = "1b84eb9e366d88cef5a285baa0047a6b123e95c9"
ZED_FAMILIES = ["one", "ayu", "gruvbox"]

ROOT = Path(__file__).resolve().parent.parent
THEMES_DIR = ROOT / "assets" / "themes"


def fetch(url: str, attempts: int = 4) -> bytes:
  """GET with a short backoff; raw.githubusercontent.com throttles bursts."""
  request = urllib.request.Request(url, headers={"User-Agent": "deathpush-vendor-themes"})
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


def download(url: str) -> bytes:
  """Fetch a file and include its URL in any failure message."""
  try:
    return fetch(url)
  except Exception as error:
    raise RuntimeError(f"failed to fetch {url}: {error}") from error


def validate_theme_family(family: str, source_url: str, content: bytes) -> list[tuple[str, str]]:
  """Validate the Zed family envelope and return each theme's name and appearance."""
  try:
    document = json.loads(content)
  except (UnicodeDecodeError, json.JSONDecodeError) as error:
    raise ValueError(f"{family}: {source_url} is not valid JSON: {error}") from error
  if not isinstance(document, dict):
    raise ValueError(f"{family}: {source_url} must contain a JSON object")

  themes = document.get("themes")
  if not isinstance(themes, list):
    raise ValueError(f"{family}: {source_url} must contain a top-level themes array")

  summary: list[tuple[str, str]] = []
  for index, theme in enumerate(themes):
    if not isinstance(theme, dict):
      raise ValueError(f"{family}: theme {index} must be an object")
    name = theme.get("name")
    appearance = theme.get("appearance")
    style = theme.get("style")
    if not isinstance(name, str) or not name:
      raise ValueError(f"{family}: theme {index} must have a name")
    if not isinstance(appearance, str) or appearance not in {"dark", "light"}:
      raise ValueError(f"{family}: theme {name!r} must have appearance 'dark' or 'light'")
    if not isinstance(style, dict):
      raise ValueError(f"{family}: theme {name!r} must have a style object")
    summary.append((name, appearance))
  return summary


def vendor_themes() -> None:
  """Fetch, validate, and write the pinned theme families."""
  files: dict[str, bytes] = {}
  licenses: list[tuple[str, str, bytes]] = []
  summaries: list[tuple[str, list[tuple[str, str]]]] = []

  for family in ZED_FAMILIES:
    theme_url = f"https://raw.githubusercontent.com/{ZED_REPO}/{ZED_REV}/assets/themes/{family}/{family}.json"
    license_url = f"https://raw.githubusercontent.com/{ZED_REPO}/{ZED_REV}/assets/themes/{family}/LICENSE"
    theme_content = download(theme_url)
    files[f"{family}.json"] = theme_content
    licenses.append((family.title(), license_url, download(license_url)))
    summaries.append((family, validate_theme_family(family, theme_url, theme_content)))

  warm_theme_url = f"https://raw.githubusercontent.com/{WARM_BURNOUT_REPO}/{WARM_BURNOUT_REV}/zed/themes/warm-burnout.json"
  warm_license_url = f"https://raw.githubusercontent.com/{WARM_BURNOUT_REPO}/{WARM_BURNOUT_REV}/zed/LICENSE"
  warm_theme_content = download(warm_theme_url)
  files["warm-burnout.json"] = warm_theme_content
  licenses.append(("Warm Burnout", warm_license_url, download(warm_license_url)))
  summaries.append(("warm-burnout", validate_theme_family("warm-burnout", warm_theme_url, warm_theme_content)))

  # Nothing on disk is removed until every download and validation has completed.
  license_sections: list[bytes] = []
  for family_name, source_url, license_body in licenses:
    try:
      license_body.decode("utf-8")
    except UnicodeDecodeError as error:
      raise ValueError(f"{family_name}: {source_url} is not valid UTF-8: {error}") from error
    section = f"## {family_name} — {source_url}\n\n".encode() + license_body
    if not license_body.endswith(b"\n"):
      section += b"\n"
    license_sections.append(section)
  licenses_content = b"\n".join(license_sections)

  shutil.rmtree(THEMES_DIR, ignore_errors=True)
  THEMES_DIR.mkdir(parents=True)
  for filename, content in files.items():
    (THEMES_DIR / filename).write_bytes(content)
  (THEMES_DIR / "LICENSES.md").write_bytes(licenses_content)

  total = 0
  for family, themes in summaries:
    total += len(themes)
    names = ", ".join(f"{name} ({appearance})" for name, appearance in themes)
    print(f"{family}: {names}")
  print(f"themes: {total} themes -> {THEMES_DIR.relative_to(ROOT)}")


def main() -> int:
  vendor_themes()
  return 0


if __name__ == "__main__":
  sys.exit(main())
