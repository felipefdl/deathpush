#!/bin/bash
# Install the DeathPush CLI tool to /usr/local/bin
# Creates symlinks: dp and deathpush

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
CLI_SCRIPT="$SCRIPT_DIR/dp"
INSTALL_DIR="/usr/local/bin"

if [ ! -f "$CLI_SCRIPT" ]; then
  echo "Error: CLI script not found at $CLI_SCRIPT" >&2
  exit 1
fi

echo "Installing DeathPush CLI to $INSTALL_DIR..."

# Create /usr/local/bin if it doesn't exist
if [ ! -d "$INSTALL_DIR" ]; then
  sudo mkdir -p "$INSTALL_DIR"
fi

# Install symlinks
sudo ln -sf "$CLI_SCRIPT" "$INSTALL_DIR/dp"
sudo ln -sf "$CLI_SCRIPT" "$INSTALL_DIR/deathpush"

echo "Installed successfully."
echo "  dp         -> $CLI_SCRIPT"
echo "  deathpush  -> $CLI_SCRIPT"
echo ""
echo "Usage: dp [path]"
echo "  Opens a Git repository in DeathPush."
echo "  If no path is given, opens the current directory."
