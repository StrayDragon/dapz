#!/usr/bin/env bash
# Full local verification for dapz (aligned with lspz verify-all.sh).
# fmt + clippy + test + doc + SDD validate + prek.
# Note: debugpy e2e remains `just harness` (DAP-specific).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

step() {
  echo ""
  echo "==> $*"
}

step "cargo fmt --check"
cargo fmt -- --check

step "cargo clippy --all-features"
cargo clippy --all-features -- -D warnings

step "cargo test --all-features"
cargo test --all-features

step "cargo doc --no-deps --all-features"
cargo doc --no-deps --all-features

step "cargo test --doc --all-features"
cargo test --doc --all-features

step "llman sdd validate --all --strict --no-interactive"
llman sdd validate --all --strict --no-interactive

if command -v prek >/dev/null 2>&1; then
  step "prek run --all-files"
  prek run --all-files
else
  echo "WARN  prek not installed — skip (run: just setup)"
fi

echo ""
echo "All verification steps passed."
