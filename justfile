default:
  @just --list

dev *args:
  cargo run -p deathpush -- {{args}}

build:
  cargo build --release -p deathpush

lint:
  cargo clippy --workspace --all-targets -- -D warnings

fmt:
  cargo fmt --all

fmt-check:
  cargo fmt --all --check

check:
  cargo check --workspace --all-targets

test:
  cargo test --workspace

perf-boot:
  TZ=UTC cargo test -p deathpush-core shell_env

perf-storm:
  TZ=UTC cargo test -p deathpush-core storm

release version:
  sed -i '' 's/^version = "[^"]*"/version = "{{version}}"/' Cargo.toml
  cargo update --workspace
  git add -A && git commit -m "release: v{{version}}"
  git tag "v{{version}}"
  git push origin main --tags
