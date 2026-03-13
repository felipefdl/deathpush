#!/bin/bash
set -euo pipefail

# Source cargo environment (Xcode shell doesn't inherit user PATH)
if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

REPO_ROOT="$SRCROOT/.."
FFI_CRATE="$REPO_ROOT/crates/deathpush-ffi"
TARGET="aarch64-apple-darwin"

# Determine Rust build profile from Xcode configuration
if [ "${CONFIGURATION}" = "Debug" ]; then
  RUST_PROFILE="debug"
  CARGO_FLAGS=""
else
  RUST_PROFILE="release"
  CARGO_FLAGS="--release"
fi

RUST_LIB_DIR="$REPO_ROOT/target/$TARGET/$RUST_PROFILE"

# Build the Rust FFI library
echo "Building deathpush-ffi ($RUST_PROFILE) for $TARGET..."
cargo build \
  --manifest-path "$FFI_CRATE/Cargo.toml" \
  $CARGO_FLAGS \
  --target "$TARGET"

# Move the dylib out of the linker search path so Xcode links the static lib.
# The dylib is still needed by generate-bindings.sh for UniFFI metadata extraction.
DYLIB_STASH="$REPO_ROOT/target/$TARGET/$RUST_PROFILE/dylib"
mkdir -p "$DYLIB_STASH"
if [ -f "$RUST_LIB_DIR/libdeathpush_ffi.dylib" ]; then
  mv "$RUST_LIB_DIR/libdeathpush_ffi.dylib" "$DYLIB_STASH/"
fi

echo "Rust library built at: $RUST_LIB_DIR/libdeathpush_ffi.a"
