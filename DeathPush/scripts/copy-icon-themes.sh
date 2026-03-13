#!/bin/bash
set -euo pipefail

# Copy icon theme resources into the app bundle preserving directory structure.

ICONS_SRC="$SRCROOT/Resources/IconThemes"
ICONS_DEST="$BUILT_PRODUCTS_DIR/$UNLOCALIZED_RESOURCES_FOLDER_PATH/IconThemes"

if [ ! -d "$ICONS_SRC" ]; then
  echo "warning: Icon theme resources not found at $ICONS_SRC"
  exit 0
fi

echo "Copying icon theme resources to $ICONS_DEST..."
rm -rf "$ICONS_DEST"
mkdir -p "$ICONS_DEST"
cp -R "$ICONS_SRC/" "$ICONS_DEST/"

echo "Icon theme resources copied successfully."
