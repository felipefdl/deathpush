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

# Re-vendor the Lucide chrome icons and the Material tree icons, then regenerate the lookup tables.
icons:
  ./scripts/vendor-icons.py
  cargo fmt --all

# Re-vendor the bundled Zed theme families.
themes:
  ./scripts/vendor-themes.py

package:
  cargo packager --release

release version:
  sed -i '' 's/^version = "[^"]*"/version = "{{version}}"/' Cargo.toml
  cargo update -w -p deathpush
  git add Cargo.toml Cargo.lock
  git commit -m "chore(release): v{{version}}"
  git tag "v{{version}}"
  @echo "Push with: git push origin HEAD --tags"
