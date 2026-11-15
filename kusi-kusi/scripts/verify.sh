#!/usr/bin/env bash
set -euo pipefail

if [[ -f .env ]]; then
  set -a
  # Local ignored environment file; never print its values.
  source .env
  set +a
fi

if [[ -n "${DATABASE_URL:-}" ]]; then
  export TEST_DATABASE_URL="$DATABASE_URL"
fi

cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace
