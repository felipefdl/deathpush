default:
  @just --list

# Build Rust FFI crate (release, native arch only)
build-rust:
  cargo build --manifest-path crates/deathpush-ffi/Cargo.toml --release --target aarch64-apple-darwin

# Build Rust FFI crate as universal binary (aarch64 + x86_64)
build-rust-universal:
  cargo build --manifest-path crates/deathpush-ffi/Cargo.toml --release --target aarch64-apple-darwin
  cargo build --manifest-path crates/deathpush-ffi/Cargo.toml --release --target x86_64-apple-darwin
  mkdir -p target/universal-apple-darwin/release
  lipo -create \
    target/aarch64-apple-darwin/release/libdeathpush_ffi.a \
    target/x86_64-apple-darwin/release/libdeathpush_ffi.a \
    -output target/universal-apple-darwin/release/libdeathpush_ffi.a

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

# Bump version in all project files (Cargo.toml, Xcode project)
bump-version version:
  sed -i '' 's/^version = ".*"/version = "{{version}}"/' Cargo.toml
  sed -i '' 's/MARKETING_VERSION = .*;/MARKETING_VERSION = {{version}};/g' DeathPush/DeathPush.xcodeproj/project.pbxproj

# Tag and push a release (bumps version, commits, tags, pushes)
release version:
  just bump-version {{version}}
  git add -A && git commit -m "release: v{{version}}"
  git tag "v{{version}}"
  git push origin main --tags
