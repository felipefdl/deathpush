default:
  @just --list

# Build Rust FFI crate (release)
build-rust:
  cargo build --manifest-path crates/deathpush-ffi/Cargo.toml --release --target aarch64-apple-darwin

# Run clippy on all crates
lint:
  cargo clippy --workspace -- -D warnings

# Format Rust code
fmt:
  cargo fmt --all

# Check Rust code compiles
check:
  cargo check --workspace

# Run Rust tests
test:
  cargo test --workspace

# Create distributable DMG (requires built .app in DeathPush/build/)
dmg:
  DeathPush/scripts/create-dmg.sh

# Notarize a build artifact
notarize path:
  DeathPush/scripts/notarize.sh {{path}}

# Sign DMG with Sparkle and generate appcast
sparkle-sign path download_url_prefix="":
  DeathPush/scripts/sparkle-sign.sh {{path}} {{download_url_prefix}}

# Tag and push a release
release version:
  git add -A && git commit -m "release: v{{version}}"
  git tag "v{{version}}"
  git push origin main --tags
