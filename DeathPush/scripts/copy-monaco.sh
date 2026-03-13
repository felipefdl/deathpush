#!/bin/bash
set -euo pipefail

# Copy Monaco editor resources into the app bundle preserving directory structure.
# Xcode's default resource copying flattens directories, so we do this manually.

MONACO_SRC="$SRCROOT/Resources/Monaco"
MONACO_DEST="$BUILT_PRODUCTS_DIR/$UNLOCALIZED_RESOURCES_FOLDER_PATH/Monaco"

if [ ! -d "$MONACO_SRC" ]; then
  echo "warning: Monaco resources not found at $MONACO_SRC"
  exit 0
fi

echo "Copying Monaco resources to $MONACO_DEST..."
rm -rf "$MONACO_DEST"
mkdir -p "$MONACO_DEST"
cp -R "$MONACO_SRC/" "$MONACO_DEST/"

echo "Monaco resources copied successfully."
