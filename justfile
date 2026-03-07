default:
  @just --list

dev:
  npm run tauri dev -- --features devtools

build:
  npm run tauri build

lint:
  npx oxlint src/
  cd src-tauri && cargo clippy -- -D warnings

fmt:
  cd src-tauri && cargo fmt

check:
  cd src-tauri && cargo check

test:
  npx vitest run
  cd src-tauri && cargo test

test-watch:
  npx vitest

release version:
  sed -i '' 's/"version": "[^"]*"/"version": "{{version}}"/' package.json
  sed -i '' 's/"version": "[^"]*"/"version": "{{version}}"/' src-tauri/tauri.conf.json
  sed -i '' 's/^version = "[^"]*"/version = "{{version}}"/' src-tauri/Cargo.toml
  cargo generate-lockfile --manifest-path src-tauri/Cargo.toml
  git add -A && git commit -m "release: v{{version}}"
  git tag "v{{version}}"
  @echo "Run: git push origin main --tags"
