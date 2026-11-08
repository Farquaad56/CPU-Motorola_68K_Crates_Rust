#!/usr/bin/env bash
# Full check before tagging a release. Stops at the first failure.
# The full SingleStepTests run (317500 tests) takes a few minutes in release mode.
set -eu
cd "$(dirname "$0")/.."

# Git Bash on Windows does not always have the Rust toolchain in its PATH.
if ! command -v cargo >/dev/null 2>&1; then
  export PATH="${HOME:-}/.cargo/bin:${USERPROFILE:-}/.cargo/bin:$PATH"
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "error: cargo not found (install Rust from https://rustup.rs)" >&2
  exit 1
fi
PY=python
command -v python >/dev/null 2>&1 || PY=python3

step() { echo; echo "=== $*"; }

step "cargo fmt --check"
cargo fmt --check

step "cargo clippy --workspace --all-targets -- -D warnings"
cargo clippy --workspace --all-targets -- -D warnings

step "cargo test --workspace"
cargo test --workspace

step "hygiene"
bash tools/hygiene.sh

step "cargo build --release --workspace"
cargo build --release --workspace

step "SingleStepTests (strict, compared with tests/baseline.json)"
cargo run --release -p puce-m68k-cli -- ssc tests/singlestep --strict --compare tests/baseline.json

echo
echo "release_check: all checks passed"
