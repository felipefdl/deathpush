#!/bin/bash
# Notarize a DeathPush build for distribution outside the App Store.
#
# Prerequisites:
#   - App signed with "Developer ID Application" certificate
#   - App-specific password stored in keychain:
#     xcrun notarytool store-credentials "AC_PASSWORD" \
#       --apple-id "your@email.com" --team-id "TEAM_ID" --password "app-specific-password"
#
# Usage: ./notarize.sh <path-to-app-or-dmg>

set -euo pipefail

ARTIFACT="${1:-}"

if [ -z "$ARTIFACT" ]; then
  echo "Usage: $0 <path-to-app-or-dmg>" >&2
  echo "" >&2
  echo "Examples:" >&2
  echo "  $0 build/DeathPush.dmg" >&2
  echo "  $0 build/DeathPush.app" >&2
  exit 1
fi

if [ ! -e "$ARTIFACT" ]; then
  echo "Error: $ARTIFACT not found" >&2
  exit 1
fi

# If it's a .app, zip it first for submission
SUBMIT_PATH="$ARTIFACT"
TEMP_ZIP=""
if [[ "$ARTIFACT" == *.app ]]; then
  TEMP_ZIP="$(mktemp -t deathpush).zip"
  echo "Zipping app for submission..."
  ditto -c -k --keepParent "$ARTIFACT" "$TEMP_ZIP"
  SUBMIT_PATH="$TEMP_ZIP"
fi

echo "Submitting for notarization..."
xcrun notarytool submit "$SUBMIT_PATH" \
  --keychain-profile "AC_PASSWORD" \
  --wait

echo ""
echo "Stapling notarization ticket..."
xcrun stapler staple "$ARTIFACT"

echo ""
echo "Verifying..."
spctl -a -vvv -t install "$ARTIFACT" 2>&1 || true

# Cleanup
if [ -n "$TEMP_ZIP" ]; then
  rm -f "$TEMP_ZIP"
fi

echo ""
echo "Done. $ARTIFACT is notarized and ready for distribution."
