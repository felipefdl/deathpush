#!/bin/bash
# Sign a DMG with Sparkle and generate appcast.xml.
#
# Local usage (reads EdDSA key from Keychain):
#   ./sparkle-sign.sh build/DeathPush.dmg
#
# CI usage (reads key from env var):
#   SPARKLE_KEY_FILE=path/to/key ./sparkle-sign.sh build/DeathPush.dmg
#
# Output: appcast.xml in the same directory as the DMG.
# Use --download-url-prefix to set the base URL for DMG downloads (required for hosted appcasts).

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$SCRIPT_DIR/.."

DMG_PATH="${1:-}"
DOWNLOAD_URL_PREFIX="${2:-}"

if [ -z "$DMG_PATH" ]; then
  echo "Usage: $0 <path-to-dmg> [download-url-prefix]" >&2
  echo "" >&2
  echo "Examples:" >&2
  echo "  $0 build/DeathPush.dmg" >&2
  echo "  $0 build/DeathPush.dmg https://github.com/felipefdl/deathpush/releases/download/v1.0.0/" >&2
  exit 1
fi

if [ ! -f "$DMG_PATH" ]; then
  echo "Error: $DMG_PATH not found" >&2
  exit 1
fi

# Locate Sparkle tools from SPM build artifacts
SPARKLE_BIN=$(find ~/Library/Developer/Xcode/DerivedData -path "*/artifacts/sparkle/Sparkle/bin" -type d 2>/dev/null | head -1)

if [ -z "$SPARKLE_BIN" ]; then
  echo "Error: Sparkle bin tools not found in DerivedData." >&2
  echo "Build the project in Xcode first to download the Sparkle package." >&2
  exit 1
fi

SIGN_UPDATE="$SPARKLE_BIN/sign_update"
GENERATE_APPCAST="$SPARKLE_BIN/generate_appcast"

KEY_ARGS=()
if [ -n "${SPARKLE_KEY_FILE:-}" ]; then
  KEY_ARGS=(--ed-key-file "$SPARKLE_KEY_FILE")
fi

# Sign the DMG
echo "Signing DMG with Sparkle EdDSA key..."
"$SIGN_UPDATE" "${KEY_ARGS[@]}" "$DMG_PATH"

# Generate appcast.xml in the DMG's directory
APPCAST_ARGS=("${KEY_ARGS[@]}")
if [ -n "$DOWNLOAD_URL_PREFIX" ]; then
  APPCAST_ARGS+=(--download-url-prefix "$DOWNLOAD_URL_PREFIX")
fi

DMG_DIR="$(dirname "$DMG_PATH")"

echo "Generating appcast.xml..."
"$GENERATE_APPCAST" "${APPCAST_ARGS[@]}" "$DMG_DIR"

echo ""
echo "Done."
echo "  Signed: $DMG_PATH"
echo "  Appcast: $DMG_DIR/appcast.xml"
