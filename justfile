default:
  @just --list

dev:
  npm run tauri dev

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

test-watch:
  npx vitest
