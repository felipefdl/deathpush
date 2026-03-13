#!/bin/bash
# Create a distributable DMG for DeathPush
# Requires: create-dmg (brew install create-dmg)

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."
APP_NAME="DeathPush"
APP_PATH="$PROJECT_DIR/build/$APP_NAME.app"
DMG_DIR="$PROJECT_DIR/build"
DMG_NAME="$APP_NAME.dmg"

# Check for create-dmg
if ! command -v create-dmg &>/dev/null; then
  echo "Error: create-dmg not found. Install with: brew install create-dmg" >&2
  exit 1
fi

# Check for built app
if [ ! -d "$APP_PATH" ]; then
  echo "Error: $APP_PATH not found." >&2
  echo "Build the app in Xcode first (Product > Archive, then Export)." >&2
  exit 1
fi

# Clean previous DMG
rm -f "$DMG_DIR/$DMG_NAME"

echo "Creating DMG..."

create-dmg \
  --volname "$APP_NAME" \
  --volicon "$APP_PATH/Contents/Resources/AppIcon.icns" \
  --window-pos 200 120 \
  --window-size 660 400 \
  --icon-size 100 \
  --icon "$APP_NAME.app" 180 170 \
  --hide-extension "$APP_NAME.app" \
  --app-drop-link 480 170 \
  --no-internet-enable \
  "$DMG_DIR/$DMG_NAME" \
  "$APP_PATH"

echo ""
echo "DMG created: $DMG_DIR/$DMG_NAME"
echo ""
echo "Next steps:"
echo "  1. Sign:     codesign --sign 'Developer ID Application: ...' $DMG_DIR/$DMG_NAME"
echo "  2. Notarize: xcrun notarytool submit $DMG_DIR/$DMG_NAME --apple-id YOU --team-id TEAM --password @keychain:AC_PASSWORD --wait"
echo "  3. Staple:   xcrun stapler staple $DMG_DIR/$DMG_NAME"
