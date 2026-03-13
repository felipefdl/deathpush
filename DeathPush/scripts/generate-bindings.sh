#!/bin/bash
set -euo pipefail

# Source cargo environment (Xcode shell doesn't inherit user PATH)
if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

REPO_ROOT="$SRCROOT/.."
TARGET="aarch64-apple-darwin"

if [ "${CONFIGURATION}" = "Debug" ]; then
  RUST_PROFILE="debug"
else
  RUST_PROFILE="release"
fi

DYLIB_PATH="$REPO_ROOT/target/$TARGET/$RUST_PROFILE/dylib/libdeathpush_ffi.dylib"
SWIFT_OUT="$SRCROOT/DeathPush/Bridge/Generated"
FFI_OUT="$SRCROOT/Bridge/FFI"

# Generate UniFFI Swift bindings
echo "Generating UniFFI Swift bindings..."
cargo run \
  --manifest-path "$REPO_ROOT/crates/deathpush-ffi/Cargo.toml" \
  --bin uniffi-bindgen \
  generate \
  --library "$DYLIB_PATH" \
  --language swift \
  --out-dir /tmp/deathpush-uniffi-gen

# Copy Swift source files into the app source tree
cp /tmp/deathpush-uniffi-gen/DeathPushCore.swift "$SWIFT_OUT/"
cp /tmp/deathpush-uniffi-gen/deathpush_core.swift "$SWIFT_OUT/"

# Copy headers to the FFI directory (bridging header imports these)
cp /tmp/deathpush-uniffi-gen/DeathPushFFI.h "$FFI_OUT/"
cp /tmp/deathpush-uniffi-gen/deathpush_coreFFI.h "$FFI_OUT/"

echo "UniFFI bindings generated successfully."
