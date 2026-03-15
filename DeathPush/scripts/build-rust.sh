#!/bin/bash
set -euo pipefail

# Source cargo environment (Xcode shell doesn't inherit user PATH)
if [ -f "$HOME/.cargo/env" ]; then
  source "$HOME/.cargo/env"
fi

REPO_ROOT="$SRCROOT/.."
FFI_CRATE="$REPO_ROOT/crates/deathpush-ffi"

# Determine Rust build profile from Xcode configuration
if [ "${CONFIGURATION}" = "Debug" ]; then
  RUST_PROFILE="debug"
  CARGO_FLAGS=""
else
  RUST_PROFILE="release"
  CARGO_FLAGS="--release"
fi

build_single_target() {
  local target="$1"
  local lib_dir="$REPO_ROOT/target/$target/$RUST_PROFILE"

  echo "Building deathpush-ffi ($RUST_PROFILE) for $target..."
  cargo build \
    --manifest-path "$FFI_CRATE/Cargo.toml" \
    $CARGO_FLAGS \
    --target "$target"

  # Move the dylib out of the linker search path so Xcode links the static lib.
  # The dylib is still needed by generate-bindings.sh for UniFFI metadata extraction.
  local dylib_stash="$lib_dir/dylib"
  mkdir -p "$dylib_stash"
  if [ -f "$lib_dir/libdeathpush_ffi.dylib" ]; then
    mv "$lib_dir/libdeathpush_ffi.dylib" "$dylib_stash/"
  fi
}

if [ "$RUST_PROFILE" = "release" ]; then
  # Universal binary: build both architectures, merge with lipo
  build_single_target "aarch64-apple-darwin"
  build_single_target "x86_64-apple-darwin"

  UNIVERSAL_DIR="$REPO_ROOT/target/universal-apple-darwin/$RUST_PROFILE"
  mkdir -p "$UNIVERSAL_DIR"

  lipo -create \
    "$REPO_ROOT/target/aarch64-apple-darwin/$RUST_PROFILE/libdeathpush_ffi.a" \
    "$REPO_ROOT/target/x86_64-apple-darwin/$RUST_PROFILE/libdeathpush_ffi.a" \
    -output "$UNIVERSAL_DIR/libdeathpush_ffi.a"

  echo "Universal static library at: $UNIVERSAL_DIR/libdeathpush_ffi.a"
else
  # Debug: build only for the native architecture
  build_single_target "aarch64-apple-darwin"
  echo "Rust library built at: $REPO_ROOT/target/aarch64-apple-darwin/$RUST_PROFILE/libdeathpush_ffi.a"
fi
