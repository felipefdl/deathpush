default:
  @just --list

dev:
  vp run tauri dev --features devtools

build:
  vp run tauri build

lint:
  vp lint src/
  cd src-tauri && cargo clippy -- -D warnings

fmt:
  vp fmt src vite.config.ts
  cd src-tauri && cargo fmt

check:
  vp check src vite.config.ts
  cd src-tauri && cargo check

test:
  vp test run
  cd src-tauri && cargo test

test-watch:
  vp test watch

release version:
  sed -i '' 's/"version": "[^"]*"/"version": "{{version}}"/' package.json
  sed -i '' 's/"version": "[^"]*"/"version": "{{version}}"/' src-tauri/tauri.conf.json
  sed -i '' 's/^version = "[^"]*"/version = "{{version}}"/' src-tauri/Cargo.toml
  cargo generate-lockfile --manifest-path src-tauri/Cargo.toml
  git add -A && git commit -m "release: v{{version}}"
  git tag "v{{version}}"
  git push origin main --tags
