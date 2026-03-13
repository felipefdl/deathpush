#!/bin/bash
set -euo pipefail

# Bundle Monaco editor files from node_modules into the Xcode resource directory.
# Run this manually when updating Monaco version.

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
MONACO_SRC="$REPO_ROOT/node_modules/monaco-editor/min"
THEMES_SRC="$REPO_ROOT/src/lib/themes/json"
DEST="$REPO_ROOT/DeathPush/Resources/Monaco"

if [ ! -d "$MONACO_SRC" ]; then
  echo "Error: Monaco not found at $MONACO_SRC. Run 'npm install' first."
  exit 1
fi

echo "Bundling Monaco editor..."

# Copy Monaco min build (AMD loader + editor core + languages)
rm -rf "$DEST/vs"
cp -R "$MONACO_SRC/vs" "$DEST/vs"

# Copy theme JSON files
rm -rf "$DEST/themes"
mkdir -p "$DEST/themes"
cp "$THEMES_SRC"/*.json "$DEST/themes/"

echo "Monaco bundle complete at: $DEST"
echo "  Editor: $(du -sh "$DEST/vs" | cut -f1)"
echo "  Themes: $(du -sh "$DEST/themes" | cut -f1)"
